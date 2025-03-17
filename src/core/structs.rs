use egui::ResizeDirection;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct WindowState {
    pub is_dragging: bool,
    pub is_maximized: bool,
    pub resizing: Option<ResizeDirection>,
}

#[derive(Serialize, Deserialize)]
pub struct Playlist {
    pub name: String,
    pub description: String,
    pub image_path: String,
}
