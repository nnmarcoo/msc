//! A result already in the library must be recognised as held.
//!
//! The bug this guards: YouTube gives one recording different ids depending on
//! how it was reached — searching "radiohead reckoner" answers `pYHEpDnvVPk`
//! while the same track inside the In Rainbows listing is `_uofQD-N6UI`, both
//! verified against the live API. Matching on the id therefore reported an owned
//! track as missing and downloaded a second copy under a slightly different
//! name.
//!
//! Requires `ffmpeg` on PATH to synthesize tagged audio, and is skipped without
//! it, since a missing tool is not a failing library.

#![cfg(feature = "explore")]

use std::path::PathBuf;
use std::process::Command;

use verse_core::Library;
use verse_core::explore::already_held;

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "verse-held-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).expect("a scratch directory");

        Self { root }
    }

    fn library(&self) -> Library {
        Library::open_at(&self.root.join("library.db")).expect("a library")
    }

    fn track(&self, file: &str, title: &str, artist: &str, seconds: u32) -> Option<PathBuf> {
        let path = self.root.join(file);

        let made = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "anullsrc=r=44100:cl=stereo",
                "-t",
                &seconds.to_string(),
                "-metadata",
                &format!("title={title}"),
                "-metadata",
                &format!("artist={artist}"),
                "-metadata",
                &format!("album_artist={artist}"),
                path.to_str()?,
            ])
            .output()
            .ok()?;

        (made.status.success() && path.exists()).then_some(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

fn ffmpeg_missing() -> bool {
    !Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
fn a_track_already_owned_is_recognised_whatever_id_it_arrived_under() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("owned");
    let Some(path) = scratch.track("reckoner.m4a", "Reckoner", "Radiohead", 4) else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };

    let mut library = scratch.library();
    let id = library.ingest(&path).expect("ingested");

    assert_eq!(
        already_held(&library, "Reckoner", Some("Radiohead"), Some(4)),
        Some(id),
        "the same recording under a different id must still be recognised"
    );
}

#[test]
fn a_decorated_store_title_meets_the_plain_one_on_disk() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("decorated");
    let Some(path) = scratch.track("creep.m4a", "Creep", "Radiohead", 4) else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };

    let mut library = scratch.library();
    let id = library.ingest(&path).expect("ingested");

    for title in [
        "Creep (Remastered 2016)",
        "Creep - 2016 Remaster",
        "creep",
        "CREEP",
    ] {
        assert_eq!(
            already_held(&library, title, Some("Radiohead"), Some(4)),
            Some(id),
            "{title:?} is the track already on disk"
        );
    }
}

#[test]
fn another_recording_of_the_same_song_is_not_already_held() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("variant");
    let Some(path) = scratch.track("creep.m4a", "Creep", "Radiohead", 4) else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };

    let mut library = scratch.library();
    library.ingest(&path).expect("ingested");

    for title in ["Creep (Acoustic)", "Creep (Live)", "Creep (Karaoke)"] {
        assert_eq!(
            already_held(&library, title, Some("Radiohead"), Some(4)),
            None,
            "{title:?} is a different recording and must still be downloadable"
        );
    }
}

#[test]
fn another_artists_cover_is_not_already_held() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("cover");
    let Some(path) = scratch.track("creep.m4a", "Creep", "Radiohead", 4) else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };

    let mut library = scratch.library();
    library.ingest(&path).expect("ingested");

    assert_eq!(
        already_held(&library, "Creep", Some("Halocene"), Some(4)),
        None,
        "a different artist's recording is not the one on disk"
    );
}

#[test]
fn length_never_refuses_a_track_whose_title_and_artist_match() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("length");
    let Some(path) = scratch.track("song.m4a", "A Song", "An Artist", 5) else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };

    let mut library = scratch.library();
    let id = library.ingest(&path).expect("ingested");

    assert_eq!(
        already_held(&library, "A Song", Some("An Artist"), Some(6)),
        Some(id),
        "a store's stated length and a decoded one disagree slightly"
    );

    assert_eq!(
        already_held(&library, "A Song", Some("An Artist"), Some(200)),
        Some(id),
        "YouTube states SAN MARCOS 22s shorter than the album cut; refusing on \n         length sent the user to download a duplicate"
    );
}

#[test]
fn length_chooses_between_two_copies_of_the_same_track() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("choose");
    let Some(short) = scratch.track("short.m4a", "A Song", "An Artist", 3) else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };
    let Some(long) = scratch.track("long.m4a", "A Song", "An Artist", 9) else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };

    let mut library = scratch.library();
    let short_id = library.ingest(&short).expect("ingested");
    let long_id = library.ingest(&long).expect("ingested");

    assert_eq!(
        already_held(&library, "A Song", Some("An Artist"), Some(3)),
        Some(short_id)
    );
    assert_eq!(
        already_held(&library, "A Song", Some("An Artist"), Some(9)),
        Some(long_id)
    );
}

#[test]
fn nothing_is_held_in_an_empty_library() {
    let scratch = Scratch::new("empty");
    let library = scratch.library();

    assert_eq!(
        already_held(&library, "Reckoner", Some("Radiohead"), Some(291)),
        None
    );
}
