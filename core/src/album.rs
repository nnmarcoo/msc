//! Albums, grouped from track tags rather than stored.
//!
//! Everything an album table would hold already lives on the tracks, so a
//! persisted copy could only drift out of step with them.

use std::collections::HashMap;

use crate::Track;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AlbumKey {
    pub name: String,
    pub artist: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Album {
    pub key: AlbumKey,
    pub year: Option<u32>,
    pub track_indices: Vec<usize>,
}

impl Album {
    pub fn name(&self) -> &str {
        &self.key.name
    }

    pub fn artist(&self) -> Option<&str> {
        self.key.artist.as_deref()
    }

    pub fn track_count(&self) -> usize {
        self.track_indices.len()
    }
}

fn crediting_artist(track: &Track) -> Option<&str> {
    track.album_artist().or_else(|| track.track_artist())
}

pub(crate) fn derive(tracks: &[Track]) -> Vec<Album> {
    let mut grouped: HashMap<(&str, Option<&str>), Vec<usize>> = HashMap::new();

    for (index, track) in tracks.iter().enumerate() {
        let Some(name) = track.album() else { continue };

        grouped
            .entry((name, crediting_artist(track)))
            .or_default()
            .push(index);
    }

    let mut sorted: Vec<(SortKey, Album)> = grouped
        .into_iter()
        .map(|((name, artist), mut track_indices)| {
            let key = AlbumKey {
                name: name.to_owned(),
                artist: artist.map(str::to_owned),
            };
            track_indices.sort_by_key(|&index| {
                let track = &tracks[index];
                (track.disc_number(), track.track_number(), index)
            });
            let year = track_indices.iter().find_map(|&index| tracks[index].year());
            let album = Album {
                key,
                year,
                track_indices,
            };
            (SortKey::of(&album), album)
        })
        .collect();

    sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
    sorted.into_iter().map(|(_, album)| album).collect()
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct SortKey {
    artist: String,
    untagged_year: bool,
    year: Option<u32>,
    name: String,
}

impl SortKey {
    fn of(album: &Album) -> Self {
        Self {
            artist: album.artist().unwrap_or_default().to_lowercase(),
            untagged_year: album.year.is_none(),
            year: album.year,
            name: album.name().to_lowercase(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A track carrying only the tags album grouping and ordering read.
    ///
    /// [`Track::from_path`] is the only real constructor and it reads a file, so
    /// the fields are written directly; they are `pub(crate)` and this is inside
    /// the crate.
    fn track(album: Option<&str>, album_artist: Option<&str>, year: Option<u32>) -> Track {
        Track {
            id: None,
            path: PathBuf::new(),
            missing: false,
            title: None,
            track_artist: None,
            album: album.map(str::to_owned),
            album_artist: album_artist.map(str::to_owned),
            genre: None,
            year,
            track_number: None,
            disc_number: None,
            comment: None,
            duration: 0.0,
            bit_rate: None,
            sample_rate: None,
            bit_depth: None,
            channels: None,
            rating: None,
        }
    }

    fn numbered(album: &str, disc: Option<u32>, number: Option<u32>) -> Track {
        Track {
            disc_number: disc,
            track_number: number,
            ..track(Some(album), None, None)
        }
    }

    fn names(albums: &[Album]) -> Vec<&str> {
        albums.iter().map(Album::name).collect()
    }

    #[test]
    fn albums_are_ordered_by_artist_then_year_then_name() {
        let tracks = [
            track(Some("Mezzanine"), Some("Massive Attack"), Some(1998)),
            track(Some("Blue Lines"), Some("Massive Attack"), Some(1991)),
            track(Some("Homogenic"), Some("Bjork"), Some(1997)),
        ];

        assert_eq!(
            names(&derive(&tracks)),
            ["Homogenic", "Blue Lines", "Mezzanine"]
        );
    }

    #[test]
    fn ordering_ignores_the_case_the_tags_were_written_in() {
        let tracks = [
            track(Some("bravo"), Some("aphex twin"), Some(2000)),
            track(Some("Alpha"), Some("APHEX TWIN"), Some(2000)),
        ];

        assert_eq!(
            names(&derive(&tracks)),
            ["Alpha", "bravo"],
            "case folded differently than the lowercasing the sort applies"
        );
    }

    #[test]
    fn an_album_with_no_year_sorts_after_its_artists_dated_records() {
        let tracks = [
            track(Some("Undated"), Some("One Artist"), None),
            track(Some("Later"), Some("One Artist"), Some(2010)),
            track(Some("Earlier"), Some("One Artist"), Some(1990)),
        ];

        assert_eq!(
            names(&derive(&tracks)),
            ["Earlier", "Later", "Undated"],
            "an untagged year did not sort last within its artist"
        );
    }

    #[test]
    fn an_album_with_no_artist_sorts_before_the_credited_ones() {
        let tracks = [
            track(Some("Credited"), Some("Someone"), Some(2000)),
            track(Some("Anonymous"), None, Some(2000)),
        ];

        assert_eq!(names(&derive(&tracks)), ["Anonymous", "Credited"]);
    }

    #[test]
    fn a_track_artist_credits_the_album_when_there_is_no_album_artist() {
        let mut only_track_artist = track(Some("Record"), None, Some(2000));
        only_track_artist.track_artist = Some("Fallback".to_owned());

        let albums = derive(&[only_track_artist]);

        assert_eq!(albums[0].artist(), Some("Fallback"));
    }

    #[test]
    fn the_album_artist_wins_over_the_track_artist() {
        let mut both = track(Some("Record"), Some("Album Artist"), Some(2000));
        both.track_artist = Some("Track Artist".to_owned());

        let albums = derive(&[both]);

        assert_eq!(albums[0].artist(), Some("Album Artist"));
    }

    #[test]
    fn tracks_with_no_album_tag_group_into_nothing() {
        let tracks = [track(None, Some("Someone"), Some(2000))];
        assert!(derive(&tracks).is_empty());
    }

    #[test]
    fn one_album_by_two_artists_is_two_records() {
        let tracks = [
            track(Some("Split"), Some("First"), Some(2000)),
            track(Some("Split"), Some("Second"), Some(2000)),
        ];

        assert_eq!(derive(&tracks).len(), 2, "the credit is part of the key");
    }

    #[test]
    fn an_albums_tracks_are_ordered_by_disc_then_number() {
        let tracks = [
            numbered("Record", Some(2), Some(1)),
            numbered("Record", Some(1), Some(2)),
            numbered("Record", Some(1), Some(1)),
        ];

        let albums = derive(&tracks);

        assert_eq!(
            albums[0].track_indices,
            vec![2, 1, 0],
            "tracks were not ordered by disc and track number"
        );
    }

    #[test]
    fn an_albums_year_is_the_first_one_its_tracks_carry() {
        let tracks = [
            Track {
                year: None,
                ..numbered("Record", Some(1), Some(1))
            },
            Track {
                year: Some(1994),
                ..numbered("Record", Some(1), Some(2))
            },
        ];

        assert_eq!(derive(&tracks)[0].year, Some(1994));
    }

    #[test]
    fn deriving_the_same_library_twice_gives_the_same_order() {
        let tracks = [
            track(Some("Bravo"), Some("Artist"), Some(2000)),
            track(Some("Alpha"), Some("Artist"), Some(2000)),
            track(Some("Charlie"), None, None),
            track(Some("Delta"), Some("artist"), Some(1999)),
        ];

        assert_eq!(
            names(&derive(&tracks)),
            names(&derive(&tracks)),
            "grouping through a HashMap leaked its iteration order into the result"
        );
    }
}
