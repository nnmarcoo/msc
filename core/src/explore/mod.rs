//! Finding music that is not in the library yet.
//!
//! Gated behind the `explore` feature. Downloading from YouTube is against its
//! terms of service, so a stock build carries none of this: no dependency, no
//! pane, and nothing in the interface naming the capability.
//!
//! The two halves of this have opposite failure modes, which is why they are
//! separate traits rather than one source. [`MusicSource`] reads an
//! unauthenticated JSON API whose response *shape* drifts occasionally, and a
//! drift costs a parser fix. [`DownloadSource`] resolves a stream URL, which
//! means solving a signature challenge YouTube actively changes, and that is
//! delegated to `yt-dlp` rather than reimplemented — it ships releases at a
//! cadence no library here could match.
//!
//! [`Found`] is deliberately not a [`crate::Track`]. A track is a file on disk
//! with tags read from it; a `Found` is a claim about a recording that may not
//! exist locally at all. Collapsing the two would leave the library holding
//! rows nothing can play, which is exactly what [`crate::Track::available`]
//! exists to prevent.
//!
//! A `Found` is a claim in a stronger sense than it looks: its stated duration
//! and its id do not always describe the same recording, because a VEVO-backed
//! album lists the album track's length beside the *music video*'s id. [`resolve`]
//! is where the two are reconciled, and it runs between finding a recording and
//! fetching it.

mod cache;
mod download;
mod innertube;
mod known;
mod resolve;
mod tag;

use std::path::{Path, PathBuf};

pub use download::{DownloadError, Progress, YtDlp};
pub use innertube::{Innertube, SearchError};
pub use known::{already_held, fold};
pub use resolve::{Resolved, for_download};
pub use tag::{Destination, TagError, path_for, sanitize, write_tags};

const COVER_LADDER: [u32; 6] = [60, 120, 226, 544, 1024, COVER_MAX];

pub const COVER_MAX: u32 = 3000;

pub fn cover_at_size(url: &str, edge: u32) -> String {
    let wanted = COVER_LADDER
        .iter()
        .copied()
        .find(|&step| step >= edge)
        .unwrap_or(COVER_MAX);

    resized(url, wanted)
}

fn resized(url: &str, edge: u32) -> String {
    match url.rfind("=w") {
        Some(index) => format!("{}=w{edge}-h{edge}-l90-rj", &url[..index]),
        None => url.to_owned(),
    }
}

pub async fn fetch_cover(url: &str) -> Option<Vec<u8>> {
    let bytes = reqwest::get(url).await.ok()?.bytes().await.ok()?;

    (!bytes.is_empty()).then(|| bytes.to_vec())
}

static LAST_COVER: std::sync::Mutex<Option<(String, Vec<u8>)>> = std::sync::Mutex::new(None);

pub async fn fetch_cover_for_file(url: &str) -> Option<Vec<u8>> {
    if let Some(bytes) = cached_cover(url) {
        return Some(bytes);
    }

    let full = resized(url, COVER_MAX);

    let bytes = match fetch_cover(&full).await {
        Some(bytes) => Some(bytes),
        None => fetch_cover(url).await,
    }?;

    *LAST_COVER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((url.to_owned(), bytes.clone()));

    Some(bytes)
}

fn cached_cover(url: &str) -> Option<Vec<u8>> {
    LAST_COVER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .filter(|(cached, _)| cached == url)
        .map(|(_, bytes)| bytes.clone())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_id: Option<String>,
    pub duration: Option<u32>,
    pub cover_url: Option<String>,
    pub explicit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Release {
    #[default]
    Album,
    Single,
    Ep,
}

impl Release {
    pub fn of(subtitle: &str) -> Self {
        match subtitle.trim().to_lowercase().as_str() {
            "single" => Self::Single,
            "ep" => Self::Ep,
            _ => Self::Album,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Album => "Album",
            Self::Single => "Single",
            Self::Ep => "EP",
        }
    }

    pub fn is_album(self) -> bool {
        matches!(self, Self::Album | Self::Ep)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundAlbum {
    pub id: String,
    pub release: Release,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<u32>,
    pub cover_url: Option<String>,
    pub explicit: bool,
    pub tracks: Vec<Found>,
}

pub trait MusicSource {
    fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<Found>, SearchError>> + Send;

    fn new_albums(
        &self,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<FoundAlbum>, SearchError>> + Send;

    fn search_albums(
        &self,
        query: &str,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<FoundAlbum>, SearchError>> + Send;

    fn album(&self, album_id: &str)
    -> impl Future<Output = Result<FoundAlbum, SearchError>> + Send;

    fn similar(
        &self,
        id: &str,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<Found>, SearchError>> + Send;
}

pub trait DownloadSource {
    fn fetch(
        &self,
        id: &str,
        directory: &Path,
        progress: impl FnMut(Progress) + Send,
    ) -> impl Future<Output = Result<PathBuf, DownloadError>> + Send;

    fn duration_of(&self, id: &str) -> impl Future<Output = Option<u32>> + Send;
}

#[cfg(test)]
mod tests {
    use super::Release;

    #[test]
    fn art_is_asked_for_at_or_above_the_size_it_is_drawn() {
        for (drawn, expected) in [(34, 60), (60, 60), (61, 120), (120, 120), (200, 226)] {
            let url = super::cover_at_size("https://host/a=w544-h544-l90-rj", drawn);
            assert!(
                url.ends_with(&format!("=w{expected}-h{expected}-l90-rj")),
                "drawn at {drawn}px asked for {url}"
            );
        }
    }

    #[test]
    fn a_size_beyond_the_ladder_asks_for_the_largest_step() {
        let url = super::cover_at_size("https://host/a=w60-h60-l90-rj", 999_999);
        assert!(
            url.ends_with(&format!(
                "=w{}-h{}-l90-rj",
                super::COVER_MAX,
                super::COVER_MAX
            )),
            "{url}"
        );
    }

    const _: () = assert!(
        super::COVER_MAX > 1400,
        "the host caps at the master's own size, so the ask must exceed it"
    );

    #[test]
    fn art_bound_for_a_file_is_asked_for_past_the_hosts_ceiling() {
        let embedded = super::resized("https://host/a=w544-h544-l90-rj", super::COVER_MAX);

        assert!(
            embedded.contains(&super::COVER_MAX.to_string()),
            "{embedded}"
        );
    }

    #[test]
    fn a_url_with_no_size_suffix_is_left_alone() {
        let plain = "https://host/plain.jpg";
        assert_eq!(super::cover_at_size(plain, 120), plain);
    }

    #[test]
    fn a_release_reads_its_kind_from_the_subtitle() {
        assert_eq!(Release::of("Album"), Release::Album);
        assert_eq!(Release::of("Single"), Release::Single);
        assert_eq!(Release::of("EP"), Release::Ep);
        assert_eq!(Release::of("  single  "), Release::Single);
    }

    #[test]
    fn an_unknown_subtitle_is_treated_as_an_album() {
        assert_eq!(Release::of(""), Release::Album);
        assert_eq!(Release::of("Chart"), Release::Album);
    }

    #[test]
    fn a_single_is_not_an_album() {
        assert!(Release::Album.is_album());
        assert!(Release::Ep.is_album());
        assert!(
            !Release::Single.is_album(),
            "a single is one track, and a grid of them reads as one-song albums"
        );
    }
}
