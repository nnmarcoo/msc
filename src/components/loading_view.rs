use std::path::Path;

use egui::{vec2, Align, Color32, Direction, Layout, Rect, RichText, Spinner, Ui, UiBuilder, Vec2};
use rfd::FileDialog;

use crate::{core::helps::collect_audio_files, structs::State};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct LoadingView {}

impl LoadingView {
    pub fn new() -> Self {
        LoadingView {}
    }

    pub fn show(&mut self, ui: &mut Ui, state: &mut State) {
        let desired_size = Vec2::new(200., 100.);
        let top_left = ui.min_rect().center() - desired_size * 0.5 + vec2(0., 32.); // adds half of the audio control height

        let builder = UiBuilder::new()
            .layout(Layout::top_down(Align::Center))
            .max_rect(Rect::from_min_size(top_left, desired_size))
            .id_salt(ui.make_persistent_id("centered_ui"));
        let mut child_ui = ui.new_child(builder);

        child_ui.vertical_centered(|ui| {
            ui.add(Spinner::new());
        });
    }
}
