//! Sanitized names must actually open on the filesystem underneath.
//!
//! The unit tests in `explore::tag` prove the rules agree with themselves. This
//! proves they agree with the operating system, which is the only authority
//! that matters: a reserved device name or a trailing dot is rejected by
//! Windows itself, not by anything Rust can be asked about.

#![cfg(feature = "explore")]

use std::fs;

use verse_core::explore::sanitize;

const HOSTILE: [&str; 14] = [
    "CON",
    "nul",
    "COM1",
    "aux.mp3",
    "AC/DC",
    r"back\slash",
    "trailing.",
    "trailing ",
    "a<b>c:d\"e|f?g*h",
    "line\nbreak",
    "...",
    "   ",
    "bôa",
    "東京",
];

#[test]
fn every_sanitized_name_can_be_created_and_reopened() {
    let root = std::env::temp_dir().join(format!("verse-sanitize-{}", std::process::id()));
    fs::create_dir_all(&root).expect("a scratch directory");

    for raw in HOSTILE {
        let name = sanitize(raw);
        let path = root.join(format!("{name}.m4a"));

        fs::write(&path, b"audio").unwrap_or_else(|e| {
            panic!("{raw:?} sanitized to {name:?}, which could not be written: {e}")
        });

        let read = fs::read(&path).unwrap_or_else(|e| {
            panic!("{raw:?} sanitized to {name:?}, which could not be reopened: {e}")
        });
        assert_eq!(read, b"audio");

        fs::remove_file(&path).expect("cleanup");
    }

    fs::remove_dir_all(&root).expect("cleanup");
}

#[test]
fn a_sanitized_name_is_usable_as_a_directory() {
    let root = std::env::temp_dir().join(format!("verse-sanitize-dir-{}", std::process::id()));
    fs::create_dir_all(&root).expect("a scratch directory");

    for raw in HOSTILE {
        let name = sanitize(raw);
        let path = root.join(&name).join(sanitize("In Rainbows"));

        fs::create_dir_all(&path).unwrap_or_else(|e| {
            panic!("{raw:?} sanitized to {name:?}, unusable as a directory: {e}")
        });
        assert!(path.is_dir(), "{path:?} did not survive as a directory");
    }

    fs::remove_dir_all(&root).expect("cleanup");
}

#[test]
fn a_trailing_dot_or_space_is_resolved_before_the_filesystem_sees_it() {
    assert_eq!(sanitize("Reckoner "), sanitize("Reckoner"));
    assert_eq!(sanitize("Reckoner."), sanitize("Reckoner"));
}
