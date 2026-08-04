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
//! Progress is parsed from `[download] 45.2% of ...` on stdout, which is why
//! `--newline` is passed: without it `yt-dlp` rewrites one line with carriage
//! returns and the stream yields nothing until the download ends.
//!
//! A 403 from YouTube is transient and common — it throttles by session — so a
//! failed attempt is retried against a different player client before being
//! reported. The clients are tried in the order that has proven most reliable,
//! with the default first.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::DownloadSource;

/// What to ask `yt-dlp` for, in preference order.
///
/// AAC-in-mp4 is chosen over the higher-bitrate Opus stream YouTube also
/// carries, and that is a real cost rather than an oversight: on a typical
/// track the choice is 130k AAC against 143k Opus, and Opus is the better codec
/// at equal rate. It is not selectable because verse cannot play it — kira's
/// decoder is symphonia, which ships no Opus at all, so an Opus file scans as
/// untaggable and decodes as "unsupported audio codec" whatever container it is
/// remuxed into. A download the player cannot open is worth less than one that
/// is slightly smaller.
///
/// `abr` sorts by audio bitrate so the best AAC is taken rather than whichever
/// mp4 stream is listed first, and the bare fallback exists for the rare track
/// offering no mp4 audio at all.
const FORMAT: &str = "bestaudio[ext=m4a]/bestaudio[acodec^=mp4a]/bestaudio";

const BINARY_ENV: &str = "VERSE_YT_DLP_PATH";

#[cfg(windows)]
const BINARY_NAME: &str = "yt-dlp.exe";

#[cfg(not(windows))]
const BINARY_NAME: &str = "yt-dlp";

const VENDOR_SEARCH_DEPTH: usize = 4;

const CLIENTS: [&str; 3] = ["", "android_vr", "web_safari"];

const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(750);

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
}

impl Default for YtDlp {
    fn default() -> Self {
        Self::new()
    }
}

impl YtDlp {
    pub fn new() -> Self {
        Self { binary: resolve() }
    }

    pub fn at(binary: PathBuf) -> Self {
        Self { binary }
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
            .arg("--no-part")
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
        if path.file_stem().and_then(|s| s.to_str()) == Some(id) {
            return Some(path);
        }
    }

    None
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
