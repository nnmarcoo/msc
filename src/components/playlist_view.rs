use egui::{TextureHandle, Ui};

use crate::core::Playlist::Playlist;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct PlayListView {
    playlist: Vec<Playlist>,
}

impl PlayListView {
    pub fn new() -> Self {
        PlayListView { playlist: vec![] }
    }
    pub fn show(&mut self, ui: &mut Ui) {
        for playlist in &self.playlist {}
    }
}
