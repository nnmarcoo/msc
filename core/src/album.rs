use std::collections::HashMap;

use crate::Track;

/// Identity of an album: its title plus the artist it is credited to.
///
/// The artist falls back to the track artist when no album artist is tagged.
/// Grouping on the album artist alone would scatter a compilation across one
/// album per featured performer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AlbumKey {
    pub name: String,
    pub artist: Option<String>,
}

/// Derived from the tracks that belong to it — never stored.
///
/// Every field an `albums` table would hold is already on the tracks, so a
/// persisted copy could only drift out of step with them.
#[derive(Debug, Clone)]
pub struct Album {
    pub key: AlbumKey,
    pub year: Option<u32>,
    /// Indices into [`crate::Library::tracks`], in disc/track order.
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

/// Groups tracks into albums, ordered by artist, then year, then title.
///
/// Replaces the `GROUP BY` + self-join that the `albums` table needed. Tracks
/// with no album tag are skipped rather than collected into an empty-named
/// album.
pub(crate) fn derive(tracks: &[Track]) -> Vec<Album> {
    let mut grouped: HashMap<AlbumKey, Vec<usize>> = HashMap::new();

    for (index, track) in tracks.iter().enumerate() {
        let Some(name) = track.album() else { continue };

        let key = AlbumKey {
            name: name.to_owned(),
            artist: track
                .album_artist()
                .or_else(|| track.track_artist())
                .map(str::to_owned),
        };
        grouped.entry(key).or_default().push(index);
    }

    let mut albums: Vec<Album> = grouped
        .into_iter()
        .map(|(key, mut track_indices)| {
            track_indices.sort_by_key(|&i| {
                let t = &tracks[i];
                (t.disc_number(), t.track_number(), i)
            });
            let year = track_indices.iter().find_map(|&i| tracks[i].year());
            Album {
                key,
                year,
                track_indices,
            }
        })
        .collect();

    albums.sort_by(|a, b| {
        let artist = |al: &Album| al.artist().unwrap_or_default().to_lowercase();
        artist(a)
            .cmp(&artist(b))
            // Untagged years sort last rather than first.
            .then_with(|| a.year.is_none().cmp(&b.year.is_none()))
            .then_with(|| a.year.cmp(&b.year))
            .then_with(|| a.name().to_lowercase().cmp(&b.name().to_lowercase()))
    });

    albums
}
