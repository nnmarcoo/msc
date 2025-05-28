use std::collections::HashMap;

use blake3::Hash;
use egui::ResizeDirection;

use super::{playlist::Playlist, track::Track};

#[derive(serde::Deserialize, serde::Serialize)]
pub enum View {
    Playlist,
    Settings,
    Loading,
}

impl Default for View {
    fn default() -> Self {
        View::Loading
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(skip)]
    pub is_dragging: bool,
    #[serde(skip)]
    pub is_maximized: bool,
    #[serde(skip)]
    pub resizing: Option<ResizeDirection>,
    pub audio_directory: String,
    #[serde(skip)]
    pub view: View,
    #[serde(skip)]
    pub library: HashMap<Hash, Track>,
    pub playlists: Vec<Playlist>,
}
