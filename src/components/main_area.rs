use std::collections::HashSet;

use eframe::egui::{
    pos2, scroll_area::ScrollBarVisibility, vec2, Align2, CentralPanel, Checkbox, Color32, Context, CursorIcon, DragValue, Image, Label, Response, RichText, Sense, TextEdit, TextStyle, TextWrapMode, Ui, Window
};
use egui_extras::{Column, TableBuilder};
use rfd::FileDialog;

use crate::{
    backend::{playlist::Playlist, ui::format_seconds},
    constants::HEADERS,
    msc::{State, View},
    widgets::link_label::link_label,
};

// track selections break if the user changes the search query

pub struct MainArea {
    selection: HashSet<usize>,
    show_window: bool,
}

impl MainArea {
    pub fn new() -> Self {
        MainArea {
            selection: Default::default(),
            show_window: false,
        }
    }

    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        CentralPanel::default().show(ctx, |ui| match state.view {
            View::Playlist => self.show_playlist(ctx, ui, state),
            View::_Search => self.show_search(ui, state),
            View::Settings => self.show_settings(ui, state),
            View::Library => self.show_library(ui, state),
        });
    }

    fn show_playlist(&mut self, ctx: &Context, ui: &mut Ui, state: &mut State) {
        let playlist = state
            .config
            .playlists
            .get_mut(state.selected_playlist)
            .unwrap();

        ui.horizontal(|ui| {
            if let Some(texture) = &playlist.texture {
                let image_res = ui.add(Image::new(texture).max_size(vec2(100., 100.)).sense(Sense::click()));

                if image_res.clicked() {
                    // TODO: add filter
                    if let Some(image_path) = FileDialog::new().pick_file() {
                        playlist.image_path = image_path.to_string_lossy().to_string();
                        playlist.texture = None;
                    }
                }
                image_res.on_hover_cursor(CursorIcon::PointingHand);
            }

            ui.vertical(|ui| {
                let name_res = ui.add(
                    Label::new(RichText::new(&playlist.name).strong().heading())
                        .selectable(false)
                        .sense(Sense::click()),
                );

                let desc_res = ui.add(
                    Label::new(RichText::new(&playlist.desc))
                        .selectable(false)
                        .sense(Sense::click()),
                );

                if name_res.clicked() || desc_res.clicked() {
                    self.show_window = true;
                }

                name_res.on_hover_cursor(CursorIcon::PointingHand);
                desc_res.on_hover_cursor(CursorIcon::PointingHand);
            });
        });

        if self.show_window {
            Window::new("Edit playlist")
                .open(&mut self.show_window)
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, vec2(0., 0.))
                .resizable(false)
                .default_pos(pos2(ui.available_width() / 2., ui.available_height() / 2.))
                .show(ctx, |ui| {
                    ui.add(TextEdit::singleline(&mut playlist.name).hint_text("Name"));
                    ui.add(TextEdit::singleline(&mut playlist.desc).hint_text("Description"));
                });
        } else if playlist.name.is_empty() {
            playlist.name = String::from("My Playlist");
        }
    }

    fn show_search(&mut self, ui: &mut Ui, _state: &mut State) {
        ui.heading("Search View");
    }

    fn show_settings(&mut self, ui: &mut Ui, state: &mut State) {
        ui.vertical(|ui| {
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
            ui.horizontal(|ui| {
                ui.label(RichText::new("Show Images").color(Color32::WHITE))
                    .on_hover_text("Display image metadata in the audio control bar");
                ui.add(Checkbox::new(&mut state.config.show_image, ""));
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

        let track_num_width = ui.fonts(|fonts| {
            let font_id = ui.style().text_styles[&TextStyle::Body].clone();

            fonts
                .layout_no_wrap(
                    format!("{}.", filtered_tracks.len()),
                    font_id.clone(),
                    Color32::TRANSPARENT,
                )
                .size()
                .x
        });

        // this is not correct
        let duration_width = 112. - track_num_width;

        let available_width =
            ((ui.available_width() - track_num_width - duration_width) / 3.).max(0.);

        TableBuilder::new(ui)
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
            .column(Column::exact(track_num_width))
            .column(Column::exact(available_width))
            .column(Column::exact(available_width))
            .column(Column::exact(available_width))
            .column(Column::exact(duration_width))
            .sense(Sense::click())
            .header(20., |mut header| {
                for text in HEADERS {
                    header.col(|ui| {
                        ui.strong(text);
                    });
                }
            })
            .body(|body| {
                body.rows(16., filtered_tracks.len(), |mut row| {
                    let index = row.index();
                    let track = filtered_tracks[index];

                    row.set_selected(self.selection.contains(&index));

                    row.col(|ui| {
                        ui.label(format!("{}.", index));
                    });
                    row.col(|ui| {
                        ui.add(Label::new(&track.title).wrap_mode(TextWrapMode::Truncate));
                    });
                    row.col(|ui| {
                        ui.add(Label::new(&track.artist).wrap_mode(TextWrapMode::Truncate));
                    });
                    row.col(|ui| {
                        ui.add(Label::new(&track.album).wrap_mode(TextWrapMode::Truncate));
                    });
                    row.col(|ui| {
                        ui.label(format_seconds(track.duration));
                    });

                    let response = row.response();

                    response.context_menu(|ui| {
                        ui.menu_button("Add to playlist", |ui| {
                            for playlist in &mut state.config.playlists {
                                if ui.button(&playlist.name).clicked() {
                                    if !self.selection.is_empty() {
                                        for index in &self.selection {
                                            playlist.add_track(filtered_tracks[*index].clone());
                                        }
                                    } else {
                                        playlist.add_track(track.clone());
                                    }
                                    ui.close_menu();
                                }
                            }
                        });

                        if ui.button("Add to queue").clicked() {
                            ui.close_menu();
                            state.queue.queue_track(track.clone());
                        }
                        ui.separator();

                        // should this be hidden if there is no selection?
                        if ui.button("Clear Selection").clicked() {
                            ui.close_menu();
                            self.selection.clear();
                        }

                        // should this be hidden if everything is selected?
                        if ui.button("Select all").clicked() {
                            ui.close_menu();
                            for i in 0..filtered_tracks.len() {
                                self.selection.insert(i);
                            }
                        }

                        if self.selection.is_empty() {
                            ui.separator();
                            if ui.button("Play").clicked() {
                                ui.close_menu();
                                // TODO
                            }
                            if ui.button("Play next").clicked() {
                                ui.close_menu();
                                state.queue.queue_track_next(track.clone());
                            }
                        }
                    });
                    self.toggle_row_selection(index, &response);
                });
            });
    }

    fn toggle_row_selection(&mut self, index: usize, row_response: &Response) {
        if row_response.clicked() {
            if self.selection.contains(&index) {
                self.selection.remove(&index);
            } else {
                self.selection.insert(index);
            }
        }
    }
}
