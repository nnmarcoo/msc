//! Checking that an id names the recording its listing described.
//!
//! An album listing states a track's duration and names a video id, and those
//! two fields do not always describe the same recording. YouTube Music backs a
//! VEVO-published album with the artist's *music videos*: the listing says
//! "Sorry For Party Rocking, 3:24" and the id beside it resolves to a 7:19 clip
//! with a skit either side of the song. The stated duration is the album track's
//! and is correct; the id is the video's. Nothing in the JSON marks the
//! difference — the row looks exactly like a row whose id is the audio.
//!
//! It is systematic rather than occasional. Across LMFAO's *Sorry for Party
//! Rocking*, every track published through LMFAOVEVO is minutes longer than its
//! listing claims, while the two served from `- Topic` channels match to the
//! second. An album of `- Topic` uploads — Radiohead's *In Rainbows*, say —
//! never shows it at all, which is why this can sit unnoticed for a long time
//! and then affect a whole record at once.
//!
//! Downloading the video is wrong twice over. The file is the wrong length, and
//! it is then tagged with the *listing's* metadata, so a 7:19 recording is filed
//! claiming to be 3:24 — which also means [`super::already_held`] compares a
//! future search against a duration the file does not have.
//!
//! So the id is checked before the audio is fetched, and a mismatch is repaired
//! rather than reported: the user asked for the album track, and the album track
//! is what they should get. A song search finds the same recording as an
//! audio-only upload, and that id is used instead. The substitution is carried
//! back on [`Resolved::substituted`] so a caller can say it happened — the file
//! that lands is deliberately not the one the id named, and that is worth
//! surfacing rather than hiding.
//!
//! [`TOLERANCE`] is what separates a different recording from the same one
//! measured differently. A store's copy is routinely a second or two from the
//! video's, and [`super::known`] documents genuine spreads of seventeen and
//! twenty-two seconds between two copies of one track — but those are cases of
//! *choosing between* candidates, where being wrong costs nothing. Here a false
//! positive sends a correct download through an unnecessary search, and a false
//! negative keeps the seven-minute file. The mismatches this exists for are
//! measured in minutes, so the threshold sits well above ordinary edit variance
//! and well below the gap it is looking for.
//!
//! A candidate is accepted only if its credit agrees, which [`super::known`]
//! already knows how to decide. A search for "champagne showers lmfao" answers
//! with a karaoke cover at 267s against the wanted 265s — closer on length than
//! the real track — so length alone would swap the recording for an imitation of
//! it. Artist is the field that tells them apart.
//!
//! [`for_download`] only ever redirects on positive evidence. An unknown stated
//! duration, a probe that cannot answer, a search that finds nothing better —
//! each leaves the download exactly as it was. A check that cannot see what it
//! is checking must not act, or a network failure starts silently rewriting
//! which recording the user gets.

use super::{DownloadSource, Found, MusicSource, known};

const TOLERANCE: u32 = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub id: String,
    pub substituted: Option<String>,
}

impl Resolved {
    fn keeping(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            substituted: None,
        }
    }
}

pub async fn for_download(
    source: &impl MusicSource,
    fetcher: &impl DownloadSource,
    found: &Found,
) -> Resolved {
    let Some(expected) = found.duration else {
        return Resolved::keeping(&found.id);
    };

    let Some(actual) = fetcher.duration_of(&found.id).await else {
        return Resolved::keeping(&found.id);
    };

    if within(expected, actual) {
        return Resolved::keeping(&found.id);
    }

    match search_for_match(source, found, expected).await {
        Some(id) => Resolved {
            id,
            substituted: Some(found.id.clone()),
        },
        None => Resolved::keeping(&found.id),
    }
}

async fn search_for_match(
    source: &impl MusicSource,
    found: &Found,
    expected: u32,
) -> Option<String> {
    let query = match found.artist.as_deref() {
        Some(artist) => format!("{} {artist}", found.title),
        None => found.title.clone(),
    };

    let candidates = source.search(&query, SEARCH_WIDTH).await.ok()?;

    candidates
        .into_iter()
        .filter(|candidate| candidate.id != found.id)
        .filter(|candidate| agrees(candidate, found))
        .filter(|candidate| candidate.duration.is_some_and(|had| within(expected, had)))
        .map(|candidate| candidate.id)
        .next()
}

const SEARCH_WIDTH: usize = 10;

fn within(expected: u32, actual: u32) -> bool {
    expected.abs_diff(actual) <= TOLERANCE
}

fn agrees(candidate: &Found, wanted: &Found) -> bool {
    if known::fold(&candidate.title) != known::fold(&wanted.title) {
        return false;
    }

    match (candidate.artist.as_deref(), wanted.artist.as_deref()) {
        (Some(had), Some(want)) => credits_agree(had, want),
        (_, None) => true,
        (None, Some(_)) => false,
    }
}

fn credits_agree(had: &str, wanted: &str) -> bool {
    let (had, wanted) = (known::fold(had), known::fold(wanted));

    had == wanted || had.starts_with(&wanted) || wanted.starts_with(&had)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explore::{FoundAlbum, SearchError};
    use std::path::{Path, PathBuf};

    fn found(id: &str, title: &str, artist: &str, duration: u32) -> Found {
        Found {
            id: id.to_owned(),
            title: title.to_owned(),
            artist: Some(artist.to_owned()),
            album: None,
            album_id: None,
            duration: Some(duration),
            cover_url: None,
            explicit: false,
        }
    }

    struct Catalogue(Vec<Found>);

    impl MusicSource for Catalogue {
        async fn search(&self, _query: &str, limit: usize) -> Result<Vec<Found>, SearchError> {
            Ok(self.0.iter().take(limit).cloned().collect())
        }

        async fn new_albums(&self, _limit: usize) -> Result<Vec<FoundAlbum>, SearchError> {
            Ok(Vec::new())
        }

        async fn search_albums(
            &self,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<FoundAlbum>, SearchError> {
            Ok(Vec::new())
        }

        async fn album(&self, _album_id: &str) -> Result<FoundAlbum, SearchError> {
            Err(SearchError::NoSuchAlbum("none".to_owned()))
        }

        async fn similar(&self, _id: &str, _limit: usize) -> Result<Vec<Found>, SearchError> {
            Ok(Vec::new())
        }
    }

    struct Probe(Option<u32>);

    impl DownloadSource for Probe {
        async fn fetch(
            &self,
            _id: &str,
            _directory: &Path,
            _progress: impl FnMut(super::super::Progress) + Send,
        ) -> Result<PathBuf, super::super::DownloadError> {
            unreachable!("resolving never downloads")
        }

        async fn duration_of(&self, _id: &str) -> Option<u32> {
            self.0
        }
    }

    #[tokio::test]
    async fn a_listing_that_names_a_music_video_falls_back_to_the_album_track() {
        let wanted = found("SkTt9k4Y-a8", "Sorry For Party Rocking", "LMFAO", 204);
        let catalogue = Catalogue(vec![found(
            "C-FwdnHPR3U",
            "Sorry For Party Rocking",
            "LMFAO",
            204,
        )]);

        let resolved = for_download(&catalogue, &Probe(Some(439)), &wanted).await;

        assert_eq!(resolved.id, "C-FwdnHPR3U");
        assert_eq!(resolved.substituted.as_deref(), Some("SkTt9k4Y-a8"));
    }

    #[tokio::test]
    async fn an_id_that_resolves_to_what_it_claimed_is_left_alone() {
        let wanted = found("_uofQD-N6UI", "Reckoner", "Radiohead", 291);
        let catalogue = Catalogue(vec![found("other", "Reckoner", "Radiohead", 291)]);

        let resolved = for_download(&catalogue, &Probe(Some(289)), &wanted).await;

        assert_eq!(resolved.id, "_uofQD-N6UI", "a correct id was swapped");
        assert_eq!(resolved.substituted, None);
    }

    #[tokio::test]
    async fn a_few_seconds_of_drift_is_the_same_recording() {
        let wanted = found("a", "Song", "Artist", 200);

        for actual in [195, 200, 205, 219] {
            let resolved =
                for_download(&Catalogue(Vec::new()), &Probe(Some(actual)), &wanted).await;
            assert_eq!(resolved.id, "a", "{actual}s read as a different recording");
        }
    }

    #[tokio::test]
    async fn a_karaoke_cover_is_not_accepted_as_the_recording() {
        let wanted = found("video", "Champagne Showers", "LMFAO", 265);
        let catalogue = Catalogue(vec![
            found("karaoke", "Champagne Showers", "2010s Karaoke Band", 266),
            found("real", "Champagne Showers", "LMFAO", 264),
        ]);

        let resolved = for_download(&catalogue, &Probe(Some(401)), &wanted).await;

        assert_eq!(
            resolved.id, "real",
            "a cover was closer on length and was taken for the record"
        );
    }

    #[tokio::test]
    async fn a_mismatch_with_nothing_better_to_offer_keeps_what_it_had() {
        let wanted = found("video", "Song", "Artist", 200);

        let resolved = for_download(&Catalogue(Vec::new()), &Probe(Some(500)), &wanted).await;

        assert_eq!(resolved.id, "video");
        assert_eq!(
            resolved.substituted, None,
            "nothing was substituted, so nothing should be reported as such"
        );
    }

    #[tokio::test]
    async fn a_listing_with_no_stated_duration_is_not_second_guessed() {
        let mut wanted = found("a", "Song", "Artist", 200);
        wanted.duration = None;

        let resolved = for_download(&Catalogue(Vec::new()), &Probe(Some(999)), &wanted).await;

        assert_eq!(resolved.id, "a");
    }

    #[tokio::test]
    async fn an_unresolvable_id_is_left_for_the_download_to_report() {
        let wanted = found("a", "Song", "Artist", 200);
        let catalogue = Catalogue(vec![found("b", "Song", "Artist", 200)]);

        let resolved = for_download(&catalogue, &Probe(None), &wanted).await;

        assert_eq!(resolved.id, "a");
        assert_eq!(resolved.substituted, None);
    }

    #[tokio::test]
    async fn the_same_id_is_never_offered_as_its_own_replacement() {
        let wanted = found("a", "Song", "Artist", 200);
        let catalogue = Catalogue(vec![found("a", "Song", "Artist", 200)]);

        let resolved = for_download(&catalogue, &Probe(Some(400)), &wanted).await;

        assert_eq!(resolved.id, "a");
        assert_eq!(resolved.substituted, None, "an id replaced itself");
    }

    #[test]
    fn a_credit_agrees_with_the_same_credit_named_differently() {
        assert!(credits_agree("LMFAO", "lmfao"));
        assert!(credits_agree("LMFAO", "LMFAO feat. Natalia Kills"));
        assert!(!credits_agree("2010s Karaoke Band", "LMFAO"));
    }
}
