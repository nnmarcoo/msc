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

mod cache;
mod download;
mod innertube;
mod tag;

pub use download::{DownloadError, Progress, YtDlp};
pub use innertube::{Innertube, SearchError};
pub use tag::{Destination, TagError, path_for, sanitize, write_tags};

use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundAlbum {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<u32>,
    pub cover_url: Option<String>,
    pub tracks: Vec<Found>,
}

pub trait MusicSource {
    fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<Found>, SearchError>> + Send;

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
}
