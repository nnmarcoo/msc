use eframe::egui::{Checkbox, Color32, ComboBox, DragValue, RichText, Ui};
use rfd::FileDialog;

use crate::{backend::playlist::Playlist, msc::State};

pub struct SettingsView {}

impl SettingsView {
    pub fn new() -> Self {
        SettingsView {}
    }

    pub fn show(&self, ui: &mut Ui, state: &mut State) {
        ui.vertical(|ui| {
            ui.heading("Performance");
            ui.horizontal(|ui| {
                ui.label(RichText::new("Redraw Unfocused Window").color(Color32::WHITE))
                    .on_hover_text("How often msc updates while using other apps");
                ui.add(Checkbox::new(&mut state.config.redraw, ""));
                ui.add_space(-5.);
                ui.add_enabled(
                    state.config.redraw,
                    DragValue::new(&mut state.config.redraw_time)
                        .range(0.1..=10.0)
                        .fixed_decimals(1)
                        .speed(0.01),
                )
                .on_hover_text("seconds");
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new("Image Quality").color(Color32::WHITE))
                    .on_hover_text("Display image metadata in the audio control bar");
                ComboBox::new("image_quality_combo", "").show_ui(ui, |ui| {
                    for v in ["High", "Medium", "Low"] {
                        ui.selectable_value(&mut String::new(), String::from(v), v);
                    }
                });
            });

            ui.add_space(10.);
            ui.heading("Configuration");

            ui.horizontal(|ui| {
                ui.label(RichText::new("Audio Folder").color(Color32::WHITE))
                    .on_hover_text("Folder containing all audio files");
                if ui
                    .button("🗁")
                    .on_hover_text(&state.config.audio_directory)
                    .clicked()
                {
                    if let Some(folder_path) = FileDialog::new().pick_folder() {
                        state.config.audio_directory = folder_path.to_string_lossy().to_string();
                        state.library = Playlist::from_directory(&state.config.audio_directory);
                    }
                }
            });
        });
    }
}
