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
    let mut grouped: HashMap<AlbumKey, Vec<usize>> = HashMap::new();

    for (index, track) in tracks.iter().enumerate() {
        let Some(name) = track.album() else { continue };

        let key = AlbumKey {
            name: name.to_owned(),
            artist: crediting_artist(track).map(str::to_owned),
        };
        grouped.entry(key).or_default().push(index);
    }

    let mut albums: Vec<Album> = grouped
        .into_iter()
        .map(|(key, mut track_indices)| {
            track_indices.sort_by_key(|&index| {
                let track = &tracks[index];
                (track.disc_number(), track.track_number(), index)
            });
            let year = track_indices.iter().find_map(|&index| tracks[index].year());
            Album {
                key,
                year,
                track_indices,
            }
        })
        .collect();

    albums.sort_by(|a, b| {
        let artist = |album: &Album| album.artist().unwrap_or_default().to_lowercase();
        let untagged_year_last = |album: &Album| album.year.is_none();

        artist(a)
            .cmp(&artist(b))
            .then_with(|| untagged_year_last(a).cmp(&untagged_year_last(b)))
            .then_with(|| a.year.cmp(&b.year))
            .then_with(|| a.name().to_lowercase().cmp(&b.name().to_lowercase()))
    });

    albums
}
