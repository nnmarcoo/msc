use std::collections::HashSet;

use eframe::egui::{
    scroll_area::ScrollBarVisibility, Color32, Label, RichText, Sense, TextStyle, TextWrapMode, Ui,
};
use egui_extras::{Column, TableBuilder};

use crate::{
    backend::ui::{format_seconds, toggle_row_selection},
    constants::HEADERS,
    msc::{State, View},
    widgets::link_label::link_label,
};

pub struct LibraryView {
    selection: HashSet<usize>,
}

impl LibraryView {
    pub fn new() -> Self {
        LibraryView {
            selection: HashSet::new(),
        }
    }

    pub fn show(&mut self, ui: &mut Ui, state: &mut State) {
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
                    toggle_row_selection(&mut self.selection, index, &response);
                });
            });
    }
}
