use egui::{TextureHandle, Ui};

use crate::structs::Playlist;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct PlayListView {
    playlist: Vec<Playlist>,
    #[serde(skip)]
    images: Vec<TextureHandle>,
}

impl PlayListView {
    pub fn new() -> Self {
        PlayListView {
            playlist: vec![],
            images: vec![],
        }
    }
    pub fn show(&mut self, ui: &mut Ui) {
        for playlist in &self.playlist {}
    }
}
