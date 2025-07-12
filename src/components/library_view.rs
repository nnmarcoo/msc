use std::sync::atomic::Ordering;

use egui::{
    scroll_area::ScrollBarVisibility, Align, Label, Layout, ScrollArea, Sense, TextWrapMode, Ui,
};
use egui_extras::{Column, TableBuilder};

use crate::{
    components::shared::show_loading,
    core::{helps::format_seconds, playlist::Playlist},
    state::State,
};

use super::shared::show_empty_library;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct LibraryView {}

impl LibraryView {
    pub fn new() -> Self {
        LibraryView {}
    }

    pub fn show(&mut self, ui: &mut Ui, state: &mut State) {
        if !state.library_loaded.load(Ordering::SeqCst) {
            return show_loading(ui);
        } else if state.library.is_empty() {
            return show_empty_library(ui, state);
        }

        pub const HEADERS: [&str; 4] = ["Title", "Artist", "Album", "Duration"];
        let width = ((ui.available_width() / (HEADERS.len() - 1) as f32) - 24.).max(0.);

        let row_height = 20.;
        let row_count = state.library.len();
        let tracks: Vec<_> = state.library.iter().map(|r| r.clone()).collect(); // BAD

        TableBuilder::new(ui)
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::exact(width))
            .column(Column::exact(width))
            .column(Column::exact(width))
            .column(Column::remainder())
            .sense(Sense::click())
            .header(row_height, |mut header| {
                for text in HEADERS {
                    header.col(|ui| {
                        ui.strong(text);
                    });
                }
            })
            .body(|body| {
                body.rows(row_height, row_count, |mut row| {
                    let index = row.index();
                    let track = &tracks[index];

                    row.col(|ui| {
                        ui.add(Label::new(&track.title).truncate());
                    });
                    row.col(|ui| {
                        ui.add(Label::new(&track.artist).truncate());
                    });
                    row.col(|ui| {
                        ui.add(Label::new(&track.album).truncate());
                    });
                    row.col(|ui| {
                        ui.add(
                            Label::new(format_seconds(track.duration))
                                .wrap_mode(TextWrapMode::Truncate),
                        );
                    });

                    let res = row.response();

                    res.context_menu(|ui| {
                        ui.menu_button("Add to playlist", |ui| {
                            if ui.button("Create playlist").clicked() {
                                state.playlists.push(Playlist::new(
                                    format!("Playlist #{}", state.playlists.len() + 1),
                                    "".to_string(),
                                    "".to_string(),
                                ));
                            }

                            if !state.playlists.is_empty() {
                                ui.separator();
                                ScrollArea::vertical().show(ui, |ui| {
                                    for playlist in &mut state.playlists {
                                        if ui.button(&playlist.name).clicked() {
                                            // TODO: Add track (selection) to playlist
                                        }
                                    }
                                });
                            }
                        });

                        ui.separator();

                        if ui.button("Play").clicked() {
                            state.queue.play(track.hash, &state.library);
                            ui.close_menu();
                        }
                        if ui.button("Play next").clicked() {
                            state.queue.queue_track_next(track.hash, &state.library);
                        }
                        if ui.button("Add to queue").clicked() {
                            state.queue.queue_track(track.hash, &state.library);
                            ui.close_menu();
                        }

                        ui.separator();

                        if ui.button("Clear selection").clicked() {}
                        if ui.button("Select all").clicked() {}

                        ui.separator();

                        if ui.button("Edit metadata").clicked() {
                            // Should this open a small window
                            // or be a separate View
                        }
                    });
                });
            });
    }
}
