//! Every format the library will scan must also decode.
//!
//! [`verse_core::Library`] scans by extension, and [`verse_core::Track`] reads
//! tags with lofty — neither of which involves the audio backend. A format can
//! therefore be listed, scanned, tagged and shown in every pane while being
//! silently unplayable, which is what happened to `m4a` and `aac`: kira 0.11
//! carried no AAC codec, so those tracks appeared in the library and failed the
//! moment they were played.
//!
//! This asserts the two lists agree. Requires `ffmpeg` on PATH to synthesize
//! samples, and is skipped without it, since a missing tool is not a failing
//! library.

use std::path::{Path, PathBuf};
use std::process::Command;

const CASES: [(&str, &[&str]); 6] = [
    ("mp3", &[]),
    ("flac", &[]),
    ("wav", &[]),
    ("ogg", &[]),
    ("aac", &[]),
    ("m4a", &["-c:a", "alac"]),
];

fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn sample(into: &Path, extension: &str, codec: &[&str]) -> Option<PathBuf> {
    let path = into.join(format!("sample.{extension}"));

    let mut command = Command::new("ffmpeg");
    command.args([
        "-y",
        "-f",
        "lavfi",
        "-i",
        "anullsrc=r=44100:cl=stereo",
        "-t",
        "1",
    ]);
    command.args(codec);
    command.arg(path.to_str()?);

    let made = command.output().ok()?;
    (made.status.success() && path.exists()).then_some(path)
}

#[test]
fn every_scannable_format_can_also_be_decoded() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let root = std::env::temp_dir().join(format!("verse-formats-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("a scratch directory");

    for (extension, codec) in CASES {
        let Some(path) = sample(&root, extension, codec) else {
            eprintln!("skipping .{extension}: ffmpeg could not produce a sample");
            continue;
        };

        verse_core::Track::from_path(&path)
            .unwrap_or_else(|e| panic!(".{extension} could not be scanned: {e}"));

        kira::sound::streaming::StreamingSoundData::from_file(&path).unwrap_or_else(|e| {
            panic!(
                ".{extension} scans into the library but cannot be played: {e}. \
                 Either the codec feature is missing from kira, or the extension \
                 should not be in Library::AUDIO_EXTENSIONS."
            )
        });
    }

    std::fs::remove_dir_all(&root).ok();
}
