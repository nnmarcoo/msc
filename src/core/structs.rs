use egui::ResizeDirection;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct WindowState {
    pub is_dragging: bool,
    pub is_maximized: bool,
    pub resizing: Option<ResizeDirection>,
}
