use egui::ResizeDirection;

#[derive(serde::Deserialize, serde::Serialize)]
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
    pub audio_directory: String,
    pub view: View,
}
