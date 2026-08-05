//! Fetching audio through the `yt-dlp` binary.
//!
//! Resolving a stream URL means solving a signature challenge YouTube changes
//! deliberately and often. `yt-dlp` ships releases at a cadence nothing here
//! could match, so it is invoked rather than reimplemented, and it is found on
//! PATH rather than bundled: a tool whose whole value is being current should be
//! updated by whoever installed it, not pinned to whatever verse shipped with.
//!
//! It is looked for in three places, in order: `VERSE_YT_DLP_PATH`, then
//! `vendor/bin` beside the running binary, then PATH. An empty variable is
//! ignored rather than treated as a path, since an unset and a blank one mean
//! the same thing to whoever exported it wrong.
//!
//! The vendor lookup is what makes `scripts/setup-explore.ps1` sufficient on its
//! own. That script also sets the variable, but a variable only reaches
//! processes started after it — launch verse from an IDE, a shortcut, or a shell
//! that was already open, and the download it just installed would report itself
//! missing while sitting in the repository. Finding it by path costs a `is_file`
//! at startup and removes a whole class of "but I installed it" report.
//!
//! The search walks up from the executable rather than from the working
//! directory, which is wherever the user happened to launch from and says
//! nothing about where verse lives. `target/debug/verse.exe` puts the repository
//! root three levels up, so the depth covers a debug build, a release build, and
//! an installed layout with the binary beside `vendor`.
//!
//! `--ignore-config` is not optional. A user's own `yt-dlp.conf` is read by
//! default, and a common one carries `-x --audio-format mp3`, which silently
//! re-encodes to a lossy file at a path this module is not expecting — the
//! download appears to succeed and leaves nothing where the caller looked.
//! Every invocation is therefore built from nothing but the flags here.
//!
//! The output path is fixed by id rather than by title, so it is known before
//! the process starts. Scanning the directory afterwards for whichever file is
//! newest races every other download running beside it, and a title cannot be
//! trusted to survive `yt-dlp`'s own sanitizing intact. [`super::tag`] moves the
//! file to its library name once tagging has succeeded.
//!
//! `yt-dlp`'s default `.part` suffix is deliberately left on. Writing straight
//! to the final name makes an interrupted download indistinguishable from a
//! finished one: the existence check that skips re-fetching a file already on
//! disk would see a truncated `{id}.m4a` left by a killed process and report it
//! complete, so the user gets a corrupt file tagged and filed with no way back
//! but deleting it by hand. A partial named `.part` is unmistakably a partial —
//! [`stale_parts`] clears any before starting, since nothing here resumes one,
//! and [`written_file`] refuses to return one.
//!
//! Progress is parsed from `[download] 45.2% of ...` on stdout, which is why
//! `--newline` is passed: without it `yt-dlp` rewrites one line with carriage
//! returns and the stream yields nothing until the download ends.
//!
//! A 403 from YouTube is transient and common — it throttles by session — so a
//! failed attempt is retried against a different player client before being
//! reported. The clients are tried in the order that has proven most reliable,
//! with the default first.
//!
//! Downloads are gated to [`CONCURRENT`] at a time, and the gate lives here
//! rather than on the caller because this is the half that knows what one
//! `yt-dlp` costs: a process, a connection to a host that throttles by session,
//! and its own share of memory. Pressing download on an album asks for every
//! track at once, and ungated that spawned one process per track — twenty
//! against a twenty-track record. The throttling that produced is the same 403
//! the client retry exists to absorb, so the parallelism manufactured the
//! failure it then paid three attempts and two sleeps each to recover from.
//!
//! Three is chosen to keep the pipe busy while a track finishes without being
//! enough sessions to be throttled for it.
//!
//! The permit is held across the retry loop rather than taken per attempt. A
//! download that has already spent two clients getting to its third is the one
//! closest to finishing, and making it re-queue behind newer arrivals is how a
//! queue starves the work it has already paid for.
//!
//! A file already on disk answers before reaching the gate. It costs nothing to
//! serve and waiting for a permit to discover that would put a cache hit behind
//! three live network transfers.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Semaphore;

use super::DownloadSource;

const FORMAT: &str = "bestaudio[ext=m4a]/bestaudio[acodec^=mp4a]/bestaudio";

const BINARY_ENV: &str = "VERSE_YT_DLP_PATH";

#[cfg(windows)]
const BINARY_NAME: &str = "yt-dlp.exe";

#[cfg(not(windows))]
const BINARY_NAME: &str = "yt-dlp";

const VENDOR_SEARCH_DEPTH: usize = 4;

const CLIENTS: [&str; 3] = ["", "android_vr", "web_safari"];

const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(750);

const CONCURRENT: usize = 3;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("yt-dlp was not found at {0}")]
    NotInstalled(String),
    #[error("yt-dlp failed: {0}")]
    Failed(String),
    #[error("yt-dlp reported success but wrote no file to {0}")]
    NothingWritten(String),
    #[error("IO error: {0}")]
    Io(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    pub fraction: Option<f32>,
}

impl Progress {
    fn at(fraction: f32) -> Self {
        Self {
            fraction: Some(fraction.clamp(0.0, 1.0)),
        }
    }
}

pub struct YtDlp {
    binary: PathBuf,
    slots: Arc<Semaphore>,
}

impl Default for YtDlp {
    fn default() -> Self {
        Self::new()
    }
}

impl YtDlp {
    pub fn new() -> Self {
        Self::at(resolve())
    }

    pub fn at(binary: PathBuf) -> Self {
        Self {
            binary,
            slots: Arc::new(Semaphore::new(CONCURRENT)),
        }
    }

    pub fn binary(&self) -> &Path {
        &self.binary
    }

    pub async fn available(&self) -> bool {
        Command::new(&self.binary)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .is_ok_and(|status| status.success())
    }

    pub async fn version(&self) -> Option<String> {
        let output = Command::new(&self.binary)
            .arg("--version")
            .stdin(Stdio::null())
            .output()
            .await
            .ok()?;

        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    async fn probe_duration(&self, id: &str) -> Option<u32> {
        let output = Command::new(&self.binary)
            .arg("--ignore-config")
            .arg("--no-playlist")
            .arg("--no-warnings")
            .arg("--skip-download")
            .arg("--print")
            .arg("%(duration)s")
            .arg(format!("https://www.youtube.com/watch?v={id}"))
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        String::from_utf8_lossy(&output.stdout).trim().parse().ok()
    }

    async fn attempt(
        &self,
        id: &str,
        target: &Path,
        client: &str,
        progress: &mut (impl FnMut(Progress) + Send),
    ) -> Result<(), DownloadError> {
        let template = with_extension_template(target);

        let mut command = Command::new(&self.binary);
        command
            .arg("--ignore-config")
            .arg("--no-playlist")
            .arg("--newline")
            .arg("--no-warnings")
            .arg("-f")
            .arg(FORMAT)
            .arg("-S")
            .arg("abr,asr")
            .arg("-o")
            .arg(&template);

        if !client.is_empty() {
            command
                .arg("--extractor-args")
                .arg(format!("youtube:player_client={client}"));
        }

        command
            .arg(format!("https://www.youtube.com/watch?v={id}"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DownloadError::NotInstalled(self.binary.display().to_string())
            } else {
                DownloadError::Io(e.to_string())
            }
        })?;

        if let Some(stdout) = child.stdout.take() {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(fraction) = parse_progress(&line) {
                    progress(Progress::at(fraction));
                }
            }
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| DownloadError::Io(e.to_string()))?;

        if output.status.success() {
            return Ok(());
        }

        Err(DownloadError::Failed(first_error(
            &String::from_utf8_lossy(&output.stderr),
        )))
    }
}

impl DownloadSource for YtDlp {
    async fn duration_of(&self, id: &str) -> Option<u32> {
        self.probe_duration(id).await
    }

    async fn fetch(
        &self,
        id: &str,
        directory: &Path,
        mut progress: impl FnMut(Progress) + Send,
    ) -> Result<PathBuf, DownloadError> {
        tokio::fs::create_dir_all(directory)
            .await
            .map_err(|e| DownloadError::Io(e.to_string()))?;

        let target = directory.join(format!("{id}.m4a"));

        if tokio::fs::try_exists(&target).await.unwrap_or(false) {
            progress(Progress::at(1.0));
            return Ok(target);
        }

        let _slot = Arc::clone(&self.slots)
            .acquire_owned()
            .await
            .map_err(|_| DownloadError::Io("the download gate was closed".to_owned()))?;

        stale_parts(directory, id).await;

        let mut last = DownloadError::Failed("no attempt was made".to_owned());

        for (attempt, client) in CLIENTS.iter().enumerate() {
            if attempt > 0 {
                tokio::time::sleep(RETRY_DELAY).await;
            }

            match self.attempt(id, &target, client, &mut progress).await {
                Ok(()) => {
                    return written_file(directory, id).await.ok_or_else(|| {
                        DownloadError::NothingWritten(target.display().to_string())
                    });
                }
                Err(DownloadError::NotInstalled(path)) => {
                    return Err(DownloadError::NotInstalled(path));
                }
                Err(e) => last = e,
            }
        }

        Err(last)
    }
}

fn resolve() -> PathBuf {
    if let Some(named) = std::env::var_os(BINARY_ENV)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
    {
        return named;
    }

    vendored().unwrap_or_else(|| PathBuf::from(BINARY_NAME))
}

fn vendored() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;

    exe.ancestors()
        .skip(1)
        .take(VENDOR_SEARCH_DEPTH)
        .map(|dir| dir.join("vendor").join("bin").join(BINARY_NAME))
        .find(|candidate| candidate.is_file())
}

fn with_extension_template(target: &Path) -> PathBuf {
    target.with_extension("%(ext)s")
}

async fn written_file(directory: &Path, id: &str) -> Option<PathBuf> {
    let preferred = directory.join(format!("{id}.m4a"));
    if tokio::fs::try_exists(&preferred).await.unwrap_or(false) {
        return Some(preferred);
    }

    let mut entries = tokio::fs::read_dir(directory).await.ok()?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if is_part(&path) {
            continue;
        }
        if path.file_stem().and_then(|s| s.to_str()) == Some(id) {
            return Some(path);
        }
    }

    None
}

fn is_part(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("part"))
}

async fn stale_parts(directory: &Path, id: &str) {
    let Ok(mut entries) = tokio::fs::read_dir(directory).await else {
        return;
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();

        if is_part(&path) && starts_the_name(&path, id) {
            tokio::fs::remove_file(&path).await.ok();
        }
    }
}

fn starts_the_name(path: &Path, id: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(id))
}

fn parse_progress(line: &str) -> Option<f32> {
    let rest = line.trim().strip_prefix("[download]")?.trim_start();
    let percent = rest.split('%').next()?.trim();

    percent.parse::<f32>().ok().map(|value| value / 100.0)
}

fn first_error(stderr: &str) -> String {
    let reported: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("ERROR"))
        .collect();

    if reported.is_empty() {
        return stderr
            .lines()
            .map(str::trim)
            .rfind(|line| !line.is_empty())
            .unwrap_or("yt-dlp exited without reporting a reason")
            .to_owned();
    }

    reported.join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(line: &str, expected: f32) -> bool {
        parse_progress(line).is_some_and(|value| (value - expected).abs() < f32::EPSILON)
    }

    #[test]
    fn a_progress_line_reads_as_a_fraction() {
        assert!(close("[download]   0.7% of 4.48MiB", 0.007));
        assert!(close("[download]  44.6% of 4.48MiB", 0.446));
        assert!(close("[download] 100.0% of 4.48MiB", 1.0));
    }

    #[test]
    fn a_line_that_is_not_progress_reads_as_nothing() {
        assert_eq!(parse_progress("[FixupM4a] Correcting container"), None);
        assert_eq!(parse_progress("[download] Destination: a.m4a"), None);
        assert_eq!(parse_progress(""), None);
        assert_eq!(parse_progress("ERROR: unavailable"), None);
    }

    #[test]
    fn a_fraction_never_leaves_its_bounds() {
        assert_eq!(Progress::at(1.4).fraction, Some(1.0));
        assert_eq!(Progress::at(-0.2).fraction, Some(0.0));
    }

    #[test]
    fn the_output_template_keeps_the_id_and_lets_yt_dlp_pick_the_extension() {
        let template = with_extension_template(Path::new("/music/abc123.m4a"));

        assert!(template.ends_with("abc123.%(ext)s"), "{template:?}");
    }

    #[test]
    fn an_error_line_is_what_gets_reported() {
        let stderr = "some noise\nERROR: video unavailable\nmore noise";

        assert_eq!(first_error(stderr), "ERROR: video unavailable");
    }

    #[test]
    fn several_error_lines_are_all_reported() {
        let stderr = "ERROR: first\nnoise\nERROR: second";

        assert_eq!(first_error(stderr), "ERROR: first; ERROR: second");
    }

    #[test]
    fn a_failure_with_no_error_line_still_reports_something() {
        assert_eq!(first_error("just a warning\n"), "just a warning");
        assert!(!first_error("").is_empty());
    }

    #[test]
    fn a_vendored_binary_is_found_beside_the_executable() {
        let found = vendored();

        if let Some(path) = &found {
            assert!(path.is_file(), "{path:?} was reported but is not a file");
            assert!(path.ends_with(BINARY_NAME), "{path:?}");
        }
    }

    #[test]
    fn resolution_falls_back_to_a_bare_name_when_nothing_is_installed() {
        let resolved = resolve();

        assert!(
            resolved.is_file() || resolved == Path::new(BINARY_NAME),
            "{resolved:?} is neither an installed binary nor a name for PATH to find"
        );
    }

    #[tokio::test]
    async fn a_missing_binary_is_named_rather_than_reported_as_a_generic_failure() {
        let source = YtDlp::at(PathBuf::from("definitely-not-a-real-binary-xyz"));
        let directory = std::env::temp_dir().join("verse-dl-missing");

        let result = source.fetch("abc", &directory, |_| {}).await;

        assert!(
            matches!(result, Err(DownloadError::NotInstalled(_))),
            "got {result:?}"
        );

        tokio::fs::remove_dir_all(&directory).await.ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn no_more_than_the_gate_allows_run_at_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let source = Arc::new(YtDlp::at(PathBuf::from("unused")));
        let live = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));

        let running: Vec<_> = (0..CONCURRENT + 5)
            .map(|_| {
                let (source, live, peak) =
                    (Arc::clone(&source), Arc::clone(&live), Arc::clone(&peak));

                tokio::spawn(async move {
                    let _slot = Arc::clone(&source.slots)
                        .acquire_owned()
                        .await
                        .expect("the gate is open");

                    let now = live.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);

                    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                    live.fetch_sub(1, Ordering::SeqCst);
                })
            })
            .collect();

        for task in running {
            task.await.expect("no task panicked");
        }

        let peak = peak.load(Ordering::SeqCst);
        assert!(
            peak <= CONCURRENT,
            "{peak} downloads held the gate at once against a limit of {CONCURRENT}"
        );
        assert_eq!(
            peak, CONCURRENT,
            "the gate never filled, so nothing was measured"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_file_already_on_disk_answers_without_waiting_for_a_permit() {
        let directory =
            std::env::temp_dir().join(format!("verse-dl-nogate-{}", std::process::id()));
        tokio::fs::create_dir_all(&directory)
            .await
            .expect("scratch");
        let existing = directory.join("abc.m4a");
        tokio::fs::write(&existing, b"audio").await.expect("write");

        let source = YtDlp::at(PathBuf::from("definitely-not-a-real-binary-xyz"));
        let held = Arc::clone(&source.slots)
            .acquire_many_owned(CONCURRENT as u32)
            .await
            .expect("every permit");

        let served = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            source.fetch("abc", &directory, |_| {}),
        )
        .await
        .expect("a cache hit waited on the gate");

        assert_eq!(served.expect("the existing file"), existing);

        drop(held);
        tokio::fs::remove_dir_all(&directory).await.ok();
    }

    #[tokio::test]
    async fn a_partial_from_a_killed_download_is_swept_before_starting() {
        let directory = std::env::temp_dir().join(format!("verse-dl-part-{}", std::process::id()));
        tokio::fs::create_dir_all(&directory)
            .await
            .expect("scratch");

        let partial = directory.join("abc.m4a.part");
        tokio::fs::write(&partial, b"half").await.expect("write");
        let other = directory.join("zzz.m4a.part");
        tokio::fs::write(&other, b"half").await.expect("write");

        let source = YtDlp::at(PathBuf::from("definitely-not-a-real-binary-xyz"));
        let _ = source.fetch("abc", &directory, |_| {}).await;

        assert!(
            !tokio::fs::try_exists(&partial).await.unwrap_or(true),
            "the partial for this download survived"
        );
        assert!(
            tokio::fs::try_exists(&other).await.unwrap_or(false),
            "another download's partial was swept"
        );

        tokio::fs::remove_dir_all(&directory).await.ok();
    }

    #[tokio::test]
    async fn a_partial_is_never_returned_as_the_finished_file() {
        let directory =
            std::env::temp_dir().join(format!("verse-dl-nopart-{}", std::process::id()));
        tokio::fs::create_dir_all(&directory)
            .await
            .expect("scratch");
        tokio::fs::write(directory.join("abc.m4a.part"), b"half")
            .await
            .expect("write");

        assert_eq!(written_file(&directory, "abc").await, None);

        tokio::fs::remove_dir_all(&directory).await.ok();
    }

    #[test]
    fn a_part_file_is_told_apart_from_the_audio_beside_it() {
        assert!(is_part(Path::new("/tmp/abc.m4a.part")));
        assert!(is_part(Path::new("/tmp/abc.m4a.PART")));
        assert!(!is_part(Path::new("/tmp/abc.m4a")));
        assert!(!is_part(Path::new("/tmp/abc")));
    }

    #[tokio::test]
    async fn a_file_already_downloaded_is_not_fetched_again() {
        let directory =
            std::env::temp_dir().join(format!("verse-dl-existing-{}", std::process::id()));
        tokio::fs::create_dir_all(&directory)
            .await
            .expect("scratch");
        let existing = directory.join("abc.m4a");
        tokio::fs::write(&existing, b"audio").await.expect("write");

        let source = YtDlp::at(PathBuf::from("definitely-not-a-real-binary-xyz"));
        let mut reported = Vec::new();
        let result = source
            .fetch("abc", &directory, |p| reported.push(p))
            .await
            .expect("the existing file is returned without running yt-dlp");

        assert_eq!(result, existing);
        assert_eq!(reported.last().and_then(|p| p.fraction), Some(1.0));

        tokio::fs::remove_dir_all(&directory).await.ok();
    }
}
