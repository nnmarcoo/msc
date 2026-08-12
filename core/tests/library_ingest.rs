//! Adding one file to the library without walking the whole tree.
//!
//! Each test holds its own database in a temporary directory, since
//! [`verse_core::Library::open`] takes the single database the application owns
//! and tests sharing it would see each other's tracks.
//!
//! Requires `ffmpeg` on PATH to synthesize audio, and is skipped without it,
//! since a missing tool is not a failing library.

use std::path::{Path, PathBuf};
use std::process::Command;

use verse_core::{Library, Track};

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "verse-ingest-{name}-{}-{:?}",
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

    fn audio(&self, name: &str) -> Option<PathBuf> {
        let path = self.root.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok()?;
        }

        let made = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "anullsrc=r=44100:cl=stereo",
                "-t",
                "1",
                "-metadata",
                &format!("title={}", name.trim_end_matches(".m4a")),
                "-metadata",
                "artist=Test Artist",
                "-metadata",
                "album=Test Album",
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
fn an_ingested_file_is_in_the_library_at_once() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("one");
    let Some(path) = scratch.audio("song.m4a") else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };

    let mut library = scratch.library();
    assert!(library.is_empty(), "a fresh library holds nothing");

    let id = library.ingest(&path).expect("the file is ingested");

    assert_eq!(library.tracks().len(), 1);
    let track = library.track(id).expect("the returned id resolves");
    assert_eq!(track.path(), path);
    assert_eq!(track.title(), Some("song"));
    assert!(track.available(), "an ingested file is playable");
}

#[test]
fn an_ingested_file_joins_the_album_it_belongs_to() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("album");
    let mut library = scratch.library();

    for name in ["a.m4a", "b.m4a"] {
        let Some(path) = scratch.audio(name) else {
            eprintln!("skipping: ffmpeg could not produce a sample");
            return;
        };
        library.ingest(&path).expect("ingested");
    }

    let albums = library.albums();
    assert_eq!(albums.len(), 1, "both tracks share one album");
    assert_eq!(albums[0].track_count(), 2);
}

#[test]
fn ingesting_a_file_twice_updates_it_rather_than_duplicating_it() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("twice");
    let Some(path) = scratch.audio("song.m4a") else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };

    let mut library = scratch.library();
    let first = library.ingest(&path).expect("ingested");
    let second = library.ingest(&path).expect("ingested again");

    assert_eq!(first, second, "the same file keeps its id");
    assert_eq!(library.tracks().len(), 1, "and does not appear twice");
}

#[test]
fn a_rating_survives_the_file_being_ingested_again() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("rating");
    let Some(path) = scratch.audio("song.m4a") else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };

    let mut library = scratch.library();
    let id = library.ingest(&path).expect("ingested");

    library.set_rating(id, Some(4)).expect("rated");
    assert_eq!(library.track(id).and_then(Track::rating), Some(4));

    library.ingest(&path).expect("ingested again");

    assert_eq!(
        library.track(id).and_then(Track::rating),
        Some(4),
        "a rating is the user's, not the file's"
    );
}

#[test]
fn ingesting_does_not_repoint_the_library_root() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("root");
    let Some(path) = scratch.audio("outside/song.m4a") else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };

    let mut library = scratch.library();
    assert!(library.root().is_none());

    library.ingest(&path).expect("ingested");

    assert!(
        library.root().is_none(),
        "one ingested file is not evidence about where the collection lives"
    );
}

#[test]
fn a_file_that_is_not_audio_is_refused() {
    let scratch = Scratch::new("notaudio");
    let path = scratch.root.join("notes.txt");
    std::fs::write(&path, b"not audio").expect("write");

    let mut library = scratch.library();
    let result = library.ingest(&path);

    assert!(result.is_err(), "a text file is not a track");
    assert!(library.is_empty(), "and nothing was stored");
}

#[test]
fn a_file_with_an_audio_name_but_no_audio_in_it_is_refused() {
    let scratch = Scratch::new("corrupt");
    let path = scratch.root.join("broken.m4a");
    std::fs::write(&path, b"this is not really an m4a").expect("write");

    let mut library = scratch.library();
    let result = library.ingest(&path);

    assert!(result.is_err(), "the extension is not the evidence");
    assert!(library.is_empty(), "and nothing was stored");
}

#[test]
fn a_missing_file_is_refused() {
    let scratch = Scratch::new("missing");
    let mut library = scratch.library();

    let result = library.ingest(&scratch.root.join("nothing.m4a"));

    assert!(result.is_err());
    assert!(library.is_empty());
}

#[test]
fn what_was_ingested_survives_reopening_the_database() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("persist");
    let Some(path) = scratch.audio("song.m4a") else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };

    let id = {
        let mut library = scratch.library();
        library.ingest(&path).expect("ingested")
    };

    let reopened = scratch.library();
    let track = reopened.track(id).expect("the track is still there");

    assert_eq!(track.path(), Path::new(&path));
}

#[test]
fn a_batch_lands_every_file_and_names_each_id() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("batch");
    let mut paths = Vec::new();
    for name in ["a.m4a", "b.m4a", "c.m4a"] {
        let Some(path) = scratch.audio(name) else {
            eprintln!("skipping: ffmpeg could not produce a sample");
            return;
        };
        paths.push(path);
    }

    let mut library = scratch.library();
    let landed = library.ingest_many(&paths).expect("the batch is ingested");

    assert_eq!(landed.len(), 3, "a file was dropped from the batch");
    assert_eq!(library.tracks().len(), 3);

    for (path, id) in landed {
        let track = library.track(id).expect("the returned id resolves");
        assert_eq!(track.path(), path, "an id was paired with the wrong file");
        assert!(track.available());
    }
}

#[test]
fn a_file_that_cannot_be_read_is_skipped_rather_than_failing_the_batch() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("partial-batch");
    let Some(good) = scratch.audio("good.m4a") else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };

    let junk = scratch.root.join("junk.m4a");
    std::fs::write(&junk, b"not audio").expect("written");
    let absent = scratch.root.join("nothing.m4a");

    let mut library = scratch.library();
    let landed = library
        .ingest_many(&[good.clone(), junk, absent])
        .expect("the batch still succeeds");

    assert_eq!(landed.len(), 1, "only the readable file lands");
    assert_eq!(landed[0].0, good);
    assert_eq!(library.tracks().len(), 1);
}

#[test]
fn a_batch_of_nothing_is_not_an_error() {
    let scratch = Scratch::new("empty-batch");
    let mut library = scratch.library();

    assert!(
        library
            .ingest_many(&[])
            .expect("no batch is fine")
            .is_empty()
    );
    assert!(library.is_empty());
}

#[test]
fn a_scan_does_not_descend_into_a_dotted_folder() {
    if ffmpeg_missing() {
        eprintln!("skipping: ffmpeg is not on PATH");
        return;
    }

    let scratch = Scratch::new("hidden");
    let Some(_visible) = scratch.audio("song.m4a") else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };
    let Some(_staged) = scratch.audio(".verse-downloads/abc123.m4a") else {
        eprintln!("skipping: ffmpeg could not produce a sample");
        return;
    };

    let mut library = scratch.library();
    library.scan(&scratch.root).expect("the scan runs");

    assert_eq!(
        library.tracks().len(),
        1,
        "a staged download was ingested as a track"
    );
    assert_eq!(library.tracks()[0].title(), Some("song"));
}
