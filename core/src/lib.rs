//! Audio playback and library management.
//!
//! Three types, each owning one concern:
//!
//! - [`Library`]: the music collection, held in memory and backed by SQLite.
//! - [`Player`]: playback and the queue.
//! - [`Track`], [`Album`], [`Playlist`]: what the library is made of.
//!
//! The `explore` feature adds a fourth, [`explore`], for finding and
//! downloading music that is not in the library yet. It is off by default and
//! absent from a stock build; see that module for why.
//!
//! `Player` and `Library` are siblings, not nested. Playback methods that need
//! to resolve a track id take `&Library` explicitly:
//!
//! ```no_run
//! # use verse_core::{Library, Player};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut library = Library::open()?;
//! let mut player = Player::new()?;
//!
//! player.enqueue_all(library.available().filter_map(|t| t.id()));
//! player.play(&library)?;
//! # Ok(())
//! # }
//! ```

mod album;
mod analyzer;
mod backend;
mod db;
#[cfg(feature = "explore")]
pub mod explore;
mod library;
mod media;
mod player;
mod playlist;
mod queue;
mod track;

pub use album::{Album, AlbumKey};
pub use analyzer::{Levels, NUM_BINS, VisData, VisReader};
pub use backend::{PlaybackError, VOLUME_MAX};
pub use library::{Library, LibraryError};
pub use media::extract_artwork_bytes;
pub use player::{Player, PlayerError};
pub use playlist::Playlist;
pub use queue::{LoopMode, Queue};
pub use track::{Track, TrackError};
