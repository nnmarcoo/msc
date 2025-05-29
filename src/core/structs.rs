use std::collections::HashMap;

use blake3::Hash;
use egui::ResizeDirection;

use super::{playlist::Playlist, queue::Queue, track::Track};

#[derive(serde::Deserialize, serde::Serialize, PartialEq)]
pub enum View {
    Playlist,
    Settings,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(skip)]
    pub is_dragging: bool,
    #[serde(skip)]
    pub is_maximized: bool,
    #[serde(skip)]
    pub resizing: Option<ResizeDirection>,
    #[serde(skip)]
    pub library: HashMap<Hash, Track>,
    #[serde(skip)]
    pub is_initialized: bool,
    pub audio_directory: String,
    pub view: View,
    pub playlists: Vec<Playlist>,
    pub queue: Queue,
}

impl Default for State {
    fn default() -> Self {
        State {
            is_dragging: false,
            is_maximized: false,
            is_initialized: false,
            resizing: None,
            audio_directory: Default::default(),
            library: Default::default(),
            view: View::Playlist,
            playlists: Vec::new(),
            queue: Queue::new(),
        }
    }
}
