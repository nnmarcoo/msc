//! Tags written onto a download must read back through the same path the
//! library scans with.
//!
//! Writing a tag and reading it back with the same crate proves little; what
//! matters is that [`verse_core::Track::from_path`] — the one reader the
//! library uses — sees the fields the panes group and sort by. Requires
//! `ffmpeg` on PATH to synthesize a sample, and is skipped without it, since a
//! missing tool is not a failing library.

#![cfg(feature = "explore")]

use std::path::{Path, PathBuf};
use std::process::Command;

use verse_core::Track;
use verse_core::explore::{Destination, Found, write_tags};

fn sample(into: &Path) -> Option<PathBuf> {
    let path = into.join("sample.m4a");

    let made = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=44100:cl=stereo",
            "-t",
            "1",
            "-c:a",
            "aac",
            path.to_str()?,
        ])
        .output()
        .ok()?;

    (made.status.success() && path.exists()).then_some(path)
}

fn found() -> Found {
    Found {
        id: "xpqk9MD6vLM".to_owned(),
        title: "15 Step".to_owned(),
        artist: Some("Radiohead feat. Someone".to_owned()),
        album: Some("In Rainbows".to_owned()),
        album_id: Some("MPREb_R6C9lU4QEg2".to_owned()),
        duration: Some(238),
        cover_url: None,
        explicit: false,
    }
}

const JPEG: [u8; 8] = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];

#[test]
fn what_is_tagged_is_what_the_library_reads_back() {
    let root = std::env::temp_dir().join(format!("verse-tag-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("a scratch directory");

    let Some(path) = sample(&root) else {
        eprintln!("skipping: ffmpeg is not on PATH");
        std::fs::remove_dir_all(&root).ok();
        return;
    };

    let into = Destination {
        album_artist: Some("Radiohead".to_owned()),
        album: Some("In Rainbows".to_owned()),
        year: Some(2007),
        track_number: Some(1),
        disc_number: None,
        cover: Some(JPEG.to_vec()),
    };

    write_tags(&path, &found(), &into).expect("tags are written");

    let track = Track::from_path(&path).expect("the library reads the file");

    assert_eq!(track.title(), Some("15 Step"));
    assert_eq!(track.track_artist(), Some("Radiohead feat. Someone"));
    assert_eq!(track.album(), Some("In Rainbows"));
    assert_eq!(track.year(), Some(2007));
    assert_eq!(track.track_number(), Some(1));

    assert_eq!(
        track.album_artist(),
        Some("Radiohead"),
        "the album credit must survive, or the album splits per guest artist"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn cover_art_is_embedded_where_the_artwork_cache_looks_for_it() {
    let root = std::env::temp_dir().join(format!("verse-tag-art-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("a scratch directory");

    let Some(path) = sample(&root) else {
        eprintln!("skipping: ffmpeg is not on PATH");
        std::fs::remove_dir_all(&root).ok();
        return;
    };

    let into = Destination {
        cover: Some(JPEG.to_vec()),
        ..Destination::default()
    };

    write_tags(&path, &found(), &into).expect("tags are written");

    let art = verse_core::extract_artwork_bytes(&path);

    assert_eq!(
        art.as_deref(),
        Some(JPEG.as_slice()),
        "the cover must come back through the same reader the artwork cache uses"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_track_with_no_album_still_tags_its_title_and_artist() {
    let root = std::env::temp_dir().join(format!("verse-tag-bare-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("a scratch directory");

    let Some(path) = sample(&root) else {
        eprintln!("skipping: ffmpeg is not on PATH");
        std::fs::remove_dir_all(&root).ok();
        return;
    };

    let single = Found {
        album: None,
        album_id: None,
        artist: Some("Lone Artist".to_owned()),
        ..found()
    };

    write_tags(&path, &single, &Destination::default()).expect("tags are written");

    let track = Track::from_path(&path).expect("the library reads the file");

    assert_eq!(track.title(), Some("15 Step"));
    assert_eq!(track.track_artist(), Some("Lone Artist"));

    std::fs::remove_dir_all(&root).ok();
}
