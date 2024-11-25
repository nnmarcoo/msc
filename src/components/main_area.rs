use eframe::egui::{
    scroll_area::ScrollBarVisibility, vec2, CentralPanel, Checkbox, Color32, Context, DragValue,
    Grid, Label, RichText, ScrollArea, TextWrapMode, Ui,
};
use rfd::FileDialog;

use crate::{
    backend::{playlist::Playlist, ui::format_seconds},
    msc::{State, View},
    widgets::link_label::link_label,
};

pub struct MainArea {}

impl MainArea {
    pub fn new() -> Self {
        MainArea {}
    }

    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        CentralPanel::default().show(ctx, |ui| match state.view {
            View::Playlist => self.show_playlist(ui, state),
            View::_Search => self.show_search(ui, state),
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
                                state.library =
                                    Playlist::from_directory(&state.config.audio_directory);
                            }
                        }
                    });
                    ui.end_row();

                    ui.label(RichText::new("Show Images").color(Color32::WHITE))
                        .on_hover_text("Display image metadata in the audio control bar");
                    ui.vertical_centered(|ui| {
                        ui.add(Checkbox::new(&mut state.config.show_image, ""));
                    });
                });
        });
    }

    fn show_library(&mut self, ui: &mut Ui, state: &mut State) {
        if state.library.tracks.is_empty() {
            ui.vertical(|ui| {
                ui.add_space(ui.available_height() / 2. - 20.);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2. - 60.);
                    ui.add(Label::new("Audio folder empty!"));
                });
                ui.add_space(10.);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2. - 30.);

                    let settings_res = ui.add(link_label(
                        RichText::new("Settings").color(Color32::WHITE),
                        Color32::WHITE,
                    ));
                    if settings_res.clicked() {
                        state.view = View::Settings;
                    }
                });
            });

            return;
        }

        let column_width = ui.available_width() / 5.;

        ScrollArea::vertical()
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                Grid::new("playlist")
                    .max_col_width(column_width)
                    .min_col_width(column_width)
                    .show(ui, |ui| {
                        ui.heading("#");
                        ui.heading("Title");
                        ui.heading("Artist");
                        ui.heading("Album");
                        ui.heading("Duration");
                        ui.end_row();

                        let query = state.query.to_lowercase();

                        let filtered_tracks = if !query.is_empty() {
                            state
                                .library
                                .tracks
                                .iter()
                                .filter(|track| {
                                    track.title.to_lowercase().contains(&query)
                                        || track.artist.to_lowercase().contains(&query)
                                        || track.album.to_lowercase().contains(&query)
                                })
                                .collect::<Vec<_>>()
                        } else {
                            state.library.tracks.iter().collect::<Vec<_>>()
                        };

                        for (i, track) in filtered_tracks.iter().enumerate() {
                            ui.add(Label::new(format!("{}", i)));
                            ui.add(Label::new(&track.title).wrap_mode(TextWrapMode::Truncate));
                            ui.add(Label::new(&track.artist).wrap_mode(TextWrapMode::Truncate));
                            ui.add(Label::new(&track.album).wrap_mode(TextWrapMode::Truncate));
                            ui.label(format_seconds(track.duration));
                            ui.end_row();
                        }
                    });
            });
    }
}
