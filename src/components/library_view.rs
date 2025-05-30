use egui::{
    scroll_area::ScrollBarVisibility, Align, Label, Layout, ScrollArea, Sense, TextWrapMode, Ui,
};
use egui_extras::{Column, TableBuilder};

use crate::{core::helps::format_seconds, structs::State};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct LibraryView {}

impl LibraryView {
    pub fn new() -> Self {
        LibraryView {}
    }

    pub fn show(&mut self, ui: &mut Ui, state: &mut State) {
        pub const HEADERS: [&str; 4] = ["Title", "Artist", "Album", "Duration"];
        let width = ((ui.available_width() / (HEADERS.len() - 1) as f32) - 24.).max(0.);

        TableBuilder::new(ui)
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::exact(width))
            .column(Column::exact(width))
            .column(Column::exact(width))
            .column(Column::remainder())
            .sense(Sense::click())
            .header(20., |mut header| {
                for text in HEADERS {
                    header.col(|ui| {
                        ui.strong(text);
                    });
                }
            })
            .body(|mut body| {
                for (_, track) in &state.library {
                    body.row(20., |mut row| {
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
                                if ui.button("Create playlist").clicked() {}

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

                            if ui.button("Play").clicked() {}
                            if ui.button("Play next").clicked() {}
                            if ui.button("Add to queue").clicked() {}

                            ui.separator();

                            if ui.button("Clear selection").clicked() {}
                            if ui.button("Select all").clicked() {}
                        });
                    });
                }
            });
    }
}
