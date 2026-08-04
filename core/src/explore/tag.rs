//! Where a download is filed, and the tags written onto it.
//!
//! A download arrives as audio with no metadata, and verse's library is built
//! entirely from tags — an untagged file scans as a track with no title, no
//! artist and no album, which is to say invisible in every pane that groups by
//! any of them. Tagging is part of downloading rather than a step after it.
//!
//! [`AlbumArtist`] is always written, never left to fall back to the track
//! artist. [`crate::Album`] groups on `(album, album_artist ?? track_artist)`,
//! so an album whose tracks credit different featured artists would otherwise
//! split into one album per credit — a ten-track record arriving as six albums
//! in the grid. Writing the album's own credit onto every track is what keeps
//! it one record, and it is why [`Destination::album_artist`] exists separately
//! from the track's artist.
//!
//! The same reasoning fixes the directory: a track is filed under the album
//! artist rather than its own, so a compilation lands in one folder instead of
//! scattering across the disk by guest credit.
//!
//! [`sanitize`] is the piece most likely to produce a file that cannot be
//! opened, and it is a Windows problem specifically: the reserved device names
//! (`CON`, `NUL`, `COM1`…) fail even with an extension, a trailing dot or space
//! is silently dropped by the filesystem so two distinct titles can collide,
//! and `<>:"/\|?*` are outright illegal. It is applied on Unix too, since a
//! library synced between the two must not hold names one of them cannot open.

use std::path::{Path, PathBuf};

use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, Tag, TagExt};
use thiserror::Error;

use super::Found;

const ILLEGAL: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

const MAX_COMPONENT: usize = 120;

const UNKNOWN_ARTIST: &str = "Unknown Artist";
const UNKNOWN_ALBUM: &str = "Unknown Album";
const UNTITLED: &str = "Untitled";

#[derive(Debug, Error)]
pub enum TagError {
    #[error("Could not read {0} to tag it")]
    Unreadable(String),
    #[error("Could not write tags: {0}")]
    Write(String),
}

#[derive(Debug, Clone, Default)]
pub struct Destination {
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub cover: Option<Vec<u8>>,
}

impl Destination {
    pub fn from_album(album: &super::FoundAlbum, position: usize) -> Self {
        Self {
            album_artist: album.artist.clone(),
            album: Some(album.title.clone()),
            year: album.year,
            track_number: u32::try_from(position + 1).ok(),
            disc_number: None,
            cover: None,
        }
    }
}

pub fn path_for(root: &Path, found: &Found, into: &Destination) -> PathBuf {
    let artist = into
        .album_artist
        .as_deref()
        .or(found.artist.as_deref())
        .unwrap_or(UNKNOWN_ARTIST);

    let album = into
        .album
        .as_deref()
        .or(found.album.as_deref())
        .unwrap_or(UNKNOWN_ALBUM);

    root.join(sanitize(artist))
        .join(sanitize(album))
        .join(file_name(found, into))
}

fn file_name(found: &Found, into: &Destination) -> String {
    let title = sanitize(&found.title);

    match into.track_number {
        Some(number) => format!("{number:02} {title}.m4a"),
        None => format!("{title}.m4a"),
    }
}

pub fn sanitize(component: &str) -> String {
    let mut cleaned: String = component
        .chars()
        .map(|c| {
            if ILLEGAL.contains(&c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();

    if cleaned.chars().count() > MAX_COMPONENT {
        cleaned = cleaned.chars().take(MAX_COMPONENT).collect();
    }

    let trimmed = cleaned.trim_matches([' ', '.'].as_slice());

    let stem = trimmed
        .split_once('.')
        .map_or(trimmed, |(before, _)| before)
        .to_ascii_uppercase();

    if RESERVED.contains(&stem.as_str()) {
        return format!("_{trimmed}");
    }

    if trimmed.is_empty() {
        return UNTITLED.to_owned();
    }

    trimmed.to_owned()
}

pub fn write_tags(path: &Path, found: &Found, into: &Destination) -> Result<(), TagError> {
    let unreadable = |e: lofty::error::LoftyError| TagError::Unreadable(e.to_string());

    let mut file = Probe::open(path)
        .map_err(unreadable)?
        .read()
        .map_err(unreadable)?;

    let kind = file.primary_tag_type();
    if file.primary_tag().is_none() {
        file.insert_tag(Tag::new(kind));
    }

    let tag = file
        .primary_tag_mut()
        .ok_or_else(|| TagError::Write("the file accepted no tag".to_owned()))?;

    tag.set_title(found.title.clone());

    if let Some(artist) = found.artist.clone() {
        tag.set_artist(artist);
    }

    if let Some(album) = into.album.clone().or_else(|| found.album.clone()) {
        tag.set_album(album);
    }

    if let Some(credit) = into.album_artist.clone().or_else(|| found.artist.clone()) {
        tag.insert_text(ItemKey::AlbumArtist, credit);
    }

    if let Some(year) = into.year {
        tag.set_year(year);
    }

    if let Some(number) = into.track_number {
        tag.set_track(number);
    }

    if let Some(disc) = into.disc_number {
        tag.set_disk(disc);
    }

    if let Some(cover) = &into.cover
        && let Some(mime) = image_mime(cover)
    {
        tag.push_picture(Picture::new_unchecked(
            PictureType::CoverFront,
            Some(mime),
            None,
            cover.clone(),
        ));
    }

    tag.save_to_path(path, WriteOptions::default())
        .map_err(|e| TagError::Write(e.to_string()))
}

fn image_mime(bytes: &[u8]) -> Option<MimeType> {
    match bytes {
        [0xFF, 0xD8, 0xFF, ..] => Some(MimeType::Jpeg),
        [0x89, b'P', b'N', b'G', ..] => Some(MimeType::Png),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(title: &str) -> Found {
        Found {
            id: "abc".to_owned(),
            title: title.to_owned(),
            artist: Some("Radiohead".to_owned()),
            album: Some("In Rainbows".to_owned()),
            album_id: Some("MPRE1".to_owned()),
            duration: Some(291),
            cover_url: None,
            explicit: false,
        }
    }

    #[test]
    fn a_track_is_filed_under_its_artist_and_album() {
        let into = Destination {
            album_artist: Some("Radiohead".to_owned()),
            album: Some("In Rainbows".to_owned()),
            track_number: Some(1),
            ..Destination::default()
        };

        let path = path_for(Path::new("/music"), &found("15 Step"), &into);

        assert!(
            path.ends_with("Radiohead/In Rainbows/01 15 Step.m4a"),
            "{path:?}"
        );
    }

    #[test]
    fn a_track_number_is_padded_so_a_folder_sorts_in_order() {
        let into = Destination {
            track_number: Some(3),
            ..Destination::default()
        };
        let path = path_for(Path::new("/music"), &found("Nude"), &into);

        assert!(path.ends_with("03 Nude.m4a"), "{path:?}");
    }

    #[test]
    fn a_track_with_no_number_is_named_by_title_alone() {
        let path = path_for(Path::new("/music"), &found("Nude"), &Destination::default());

        assert!(path.ends_with("Nude.m4a"), "{path:?}");
    }

    #[test]
    fn a_guest_credit_does_not_scatter_an_album() {
        let mut guest = found("Feature");
        guest.artist = Some("Radiohead feat. Someone".to_owned());

        let into = Destination {
            album_artist: Some("Radiohead".to_owned()),
            album: Some("In Rainbows".to_owned()),
            ..Destination::default()
        };

        let path = path_for(Path::new("/music"), &guest, &into);

        assert!(path.starts_with(Path::new("/music/Radiohead")), "{path:?}");
    }

    #[test]
    fn a_track_missing_everything_still_gets_a_path() {
        let bare = Found {
            artist: None,
            album: None,
            title: String::new(),
            ..found("")
        };

        let path = path_for(Path::new("/music"), &bare, &Destination::default());

        assert!(
            path.ends_with("Unknown Artist/Unknown Album/Untitled.m4a"),
            "{path:?}"
        );
    }

    #[test]
    fn illegal_characters_become_underscores() {
        assert_eq!(sanitize("AC/DC"), "AC_DC");
        assert_eq!(sanitize(r#"a<b>c:d"e|f?g*h"#), "a_b_c_d_e_f_g_h");
        assert_eq!(sanitize(r"back\slash"), "back_slash");
    }

    #[test]
    fn trailing_dots_and_spaces_are_removed() {
        assert_eq!(sanitize("Album."), "Album");
        assert_eq!(sanitize("Album "), "Album");
        assert_eq!(sanitize("Album... "), "Album");
        assert_eq!(sanitize(" .Album. "), "Album");
    }

    #[test]
    fn reserved_device_names_are_escaped() {
        assert_eq!(sanitize("CON"), "_CON");
        assert_eq!(sanitize("nul"), "_nul");
        assert_eq!(sanitize("COM1"), "_COM1");
        assert_eq!(sanitize("aux.mp3"), "_aux.mp3");
    }

    #[test]
    fn a_name_merely_containing_a_reserved_word_is_left_alone() {
        assert_eq!(sanitize("Console"), "Console");
        assert_eq!(sanitize("Nullify"), "Nullify");
        assert_eq!(sanitize("CONCERT"), "CONCERT");
    }

    #[test]
    fn an_overlong_name_is_clamped() {
        let long = "a".repeat(400);
        let cleaned = sanitize(&long);

        assert_eq!(cleaned.chars().count(), MAX_COMPONENT);
    }

    #[test]
    fn an_overlong_name_of_wide_characters_stays_whole() {
        let long = "あ".repeat(400);
        let cleaned = sanitize(&long);

        assert_eq!(cleaned.chars().count(), MAX_COMPONENT);
        assert!(cleaned.chars().all(|c| c == 'あ'));
    }

    #[test]
    fn unicode_titles_survive_untouched() {
        assert_eq!(sanitize("bôa"), "bôa");
        assert_eq!(
            sanitize("Sigur Rós — Ágætis byrjun"),
            "Sigur Rós — Ágætis byrjun"
        );
        assert_eq!(sanitize("東京"), "東京");
    }

    #[test]
    fn a_name_that_sanitizes_to_nothing_is_still_a_name() {
        assert_eq!(sanitize("..."), UNTITLED);
        assert_eq!(sanitize("   "), UNTITLED);
        assert_eq!(sanitize(""), UNTITLED);
    }

    #[test]
    fn control_characters_are_removed() {
        assert_eq!(sanitize("line\nbreak"), "line_break");
        assert_eq!(sanitize("tab\there"), "tab_here");
    }

    #[test]
    fn cover_art_is_recognised_by_its_magic_bytes() {
        assert_eq!(
            image_mime(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]),
            Some(MimeType::Jpeg)
        );
        assert_eq!(
            image_mime(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A]),
            Some(MimeType::Png)
        );
        assert_eq!(image_mime(&[0, 1, 2, 3]), None);
        assert_eq!(image_mime(&[]), None);
    }

    #[test]
    fn a_destination_numbers_album_tracks_from_one() {
        let album = super::super::FoundAlbum {
            release: crate::explore::Release::default(),
            id: "MPRE1".to_owned(),
            title: "In Rainbows".to_owned(),
            artist: Some("Radiohead".to_owned()),
            year: Some(2007),
            cover_url: None,
            explicit: false,
            tracks: Vec::new(),
        };

        assert_eq!(Destination::from_album(&album, 0).track_number, Some(1));
        assert_eq!(Destination::from_album(&album, 9).track_number, Some(10));
        assert_eq!(Destination::from_album(&album, 0).year, Some(2007));
    }
}
