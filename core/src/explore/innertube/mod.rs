//! A minimal client for YouTube Music's Innertube API.
//!
//! Innertube is the JSON API the YouTube Music web player itself calls. It
//! needs no key and no account: a POST carrying a `context.client` block
//! identifying the caller as the web player is enough, which is what
//! [`context`] builds. Searching is a normal request/response, so unlike a
//! stdio worker there is no shared pipe to keep in step and a cancelled request
//! costs nothing.
//!
//! `params` on a search is an opaque token meaning "songs only". Without it a
//! search returns albums, artists, videos and playlists interleaved, and the
//! rows that are not songs carry no video id, so they would parse to nothing
//! and silently shorten the results. It is a constant because it encodes a
//! filter choice, not a query.
//!
//! Only the endpoints a typed query can reach are here: the two searches and
//! the album behind a result. The new-releases feed and the radio endpoint both
//! answered without being asked anything, which is not what this is for.
//!
//! Every request is cached by [`super::cache`], since typing a query issues one
//! request per keystroke after debouncing and members re-search the same artist
//! constantly.

mod nav;
mod parse;

use std::time::Duration;

use serde_json::{Value, json};
use thiserror::Error;

use super::cache::Cache;
use super::{Found, FoundAlbum, MusicSource};

const BASE: &str = "https://music.youtube.com/youtubei/v1";
const CLIENT_NAME: &str = "WEB_REMIX";
const CLIENT_VERSION: &str = "1.20240101.01.00";

const SONGS_ONLY: &str = "EgWKAQIIAWoKEAkQBRAKEAMQBA==";

const ALBUMS_ONLY: &str = "EgWKAQIYAWoKEAkQChAFEAMQBA==";

const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("YouTube Music returned {0}")]
    Status(u16),
    #[error("Could not read the response: {0}")]
    Malformed(String),
    #[error("No album with id {0}")]
    NoSuchAlbum(String),
}

pub struct Innertube {
    client: reqwest::Client,
    songs: Cache<Vec<Found>>,
    album_search: Cache<Vec<FoundAlbum>>,
    albums: Cache<FoundAlbum>,
}

impl Default for Innertube {
    fn default() -> Self {
        Self::new()
    }
}

impl Innertube {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .build()
            .unwrap_or_default();

        Self {
            client,
            songs: Cache::new(),
            album_search: Cache::new(),
            albums: Cache::new(),
        }
    }

    async fn post(&self, endpoint: &str, body: Value) -> Result<Value, SearchError> {
        let response = self
            .client
            .post(format!("{BASE}/{endpoint}?prettyPrint=false"))
            .header("Referer", "https://music.youtube.com/")
            .json(&body)
            .send()
            .await
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(SearchError::Status(status.as_u16()));
        }

        response
            .json()
            .await
            .map_err(|e| SearchError::Malformed(e.to_string()))
    }
}

fn context() -> Value {
    json!({
        "client": {
            "clientName": CLIENT_NAME,
            "clientVersion": CLIENT_VERSION,
            "hl": "en",
            "gl": "US",
        }
    })
}

fn key(query: &str, limit: usize) -> String {
    format!("{limit}\u{1}{}", query.trim().to_lowercase())
}

impl MusicSource for Innertube {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Found>, SearchError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let cached = key(query, limit);
        if let Some(hit) = self.songs.get(&cached) {
            return Ok(hit);
        }

        let response = self
            .post(
                "search",
                json!({ "context": context(), "query": query, "params": SONGS_ONLY }),
            )
            .await?;

        let found = parse::songs(&response, limit);
        self.songs.put(cached, found.clone());
        Ok(found)
    }

    async fn search_albums(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<FoundAlbum>, SearchError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let cached = key(query, limit);
        if let Some(hit) = self.album_search.get(&cached) {
            return Ok(hit);
        }

        let response = self
            .post(
                "search",
                json!({ "context": context(), "query": query, "params": ALBUMS_ONLY }),
            )
            .await?;

        let found = parse::album_results(&response, limit);
        self.album_search.put(cached, found.clone());
        Ok(found)
    }

    async fn album(&self, album_id: &str) -> Result<FoundAlbum, SearchError> {
        if let Some(hit) = self.albums.get(album_id) {
            return Ok(hit);
        }

        let response = self
            .post(
                "browse",
                json!({ "context": context(), "browseId": album_id }),
            )
            .await?;

        let album = parse::album(&response, album_id)
            .ok_or_else(|| SearchError::NoSuchAlbum(album_id.to_owned()))?;

        self.albums.put(album_id.to_owned(), album.clone());
        Ok(album)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_key_folds_case_and_padding() {
        assert_eq!(key("Radiohead", 10), key("  radiohead  ", 10));
    }

    #[test]
    fn a_limit_is_part_of_a_query_key() {
        assert_ne!(key("radiohead", 10), key("radiohead", 20));
    }

    #[test]
    fn songs_and_albums_for_one_query_do_not_share_an_entry() {
        let source = Innertube::new();
        let limit = 24;

        source.songs.put(key("in rainbows", limit), Vec::new());
        source
            .album_search
            .put(key("in rainbows", limit), vec![album_named("In Rainbows")]);

        assert_eq!(
            source.album_search.get(&key("in rainbows", limit)).as_deref(),
            Some([album_named("In Rainbows")].as_slice()),
            "the two searches a query issues collided on one key"
        );
    }

    fn album_named(title: &str) -> FoundAlbum {
        FoundAlbum {
            id: title.to_owned(),
            release: crate::explore::Release::default(),
            title: title.to_owned(),
            artist: None,
            year: None,
            cover_url: None,
            explicit: false,
            tracks: Vec::new(),
        }
    }

    #[test]
    fn the_client_context_names_the_music_player() {
        let context = context();
        assert_eq!(context["client"]["clientName"], CLIENT_NAME);
        assert_eq!(context["client"]["clientVersion"], CLIENT_VERSION);
    }
}
