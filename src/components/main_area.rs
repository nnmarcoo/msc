use eframe::egui::{vec2, CentralPanel, Checkbox, Color32, Context, DragValue, Grid, RichText, Ui};
use rfd::FileDialog;

use crate::msc::{State, View};

pub struct MainArea {}

impl MainArea {
    pub fn new() -> Self {
        MainArea {}
    }

    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        CentralPanel::default().show(ctx, |ui| match state.view {
            View::Playlist => self.show_playlist(ui, state),
            View::Search => self.show_search(ui, state),
            View::Settings => self.show_settings(ui, state),
            View::Library => self.show_library(ui, state),
        });
    }

    fn show_playlist(&mut self, ui: &mut Ui, state: &mut State) {
        ui.heading("Playlist View");
    }

    fn show_search(&mut self, ui: &mut Ui, state: &mut State) {
        ui.heading("Search View");
    }

    fn show_settings(&mut self, ui: &mut Ui, state: &mut State) {
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2. - 125.);
            Grid::new("settings")
                .spacing(vec2(30., 20.))
                .show(ui, |ui| {
                    ui.label(RichText::new("Redraw Unfocused Window").color(Color32::WHITE))
                        .on_hover_text("How often msc updates while using other apps");
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
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
                    });

                    ui.end_row();
                    ui.label(RichText::new("Audio Folder").color(Color32::WHITE))
                        .on_hover_text("Folder containing all audio files");
                    ui.vertical_centered(|ui| {
                        if ui
                            .button("🗁")
                            .on_hover_text(&state.config.audio_directory)
                            .clicked()
                        {
                            if let Some(folder_path) = FileDialog::new().pick_folder() {
                                state.config.audio_directory =
                                    folder_path.to_string_lossy().to_string();
                            }
                        }
                    });

                    ui.end_row();
                });
        });
    }

    fn show_library(&mut self, ui: &mut Ui, state: &mut State) {
        state.library.display(ui);
    }
}
