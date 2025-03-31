use egui::ResizeDirection;

// is this struct necessary
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct WindowState {
    pub is_dragging: bool,
    pub is_maximized: bool,
    pub resizing: Option<ResizeDirection>,
}
