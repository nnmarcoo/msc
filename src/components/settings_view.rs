use egui::{Color32, RichText, Ui};
use rfd::FileDialog;

use crate::structs::State;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SettingsView {}

impl SettingsView {
    pub fn new() -> Self {
        SettingsView {}
    }

    pub fn show(&mut self, ui: &mut Ui, state: &mut State) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Audio Folder").color(Color32::WHITE))
                .on_hover_text("Folder containing all audio files");
            if ui
                .button("🗁")
                .on_hover_text(&state.audio_directory)
                .clicked()
            {
                if let Some(folder_path) = FileDialog::new().pick_folder() {
                    state.audio_directory = folder_path.to_string_lossy().to_string();
                    state.start_loading_library();
                }
            }
        });
    }
}
