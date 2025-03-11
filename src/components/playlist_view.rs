use std::collections::HashSet;

use eframe::egui::{
    pos2, scroll_area::ScrollBarVisibility, vec2, Align2, Color32, Context, CursorIcon, Image,
    Label, Pos2, Rect, RichText, Sense, TextEdit, TextStyle, TextWrapMode, Ui, Window,
};
use egui_extras::{Column, TableBuilder};
use rfd::FileDialog;

use crate::{
    backend::ui::{format_seconds, toggle_row_selection},
    constants::{DEFAULT_IMAGE_IMAGE, HEADERS},
    msc::{State, View},
    widgets::link_label::link_label,
};

pub struct PlaylistView {
    show_window: bool,
    selection: HashSet<usize>,
}

impl PlaylistView {
    pub fn new() -> Self {
        PlaylistView {
            show_window: false,
            selection: HashSet::new(),
        }
    }

    pub fn show(&mut self, ctx: &Context, ui: &mut Ui, state: &mut State) {
        let playlist = state
            .config
            .playlists
            .get_mut(state.selected_playlist)
            .unwrap();

        ui.horizontal(|ui| {
            let image: Image<'_> = match &playlist.image.get_texture_medium() {
                Some(texture) => Image::new(texture),
                None => Image::new(DEFAULT_IMAGE_IMAGE),
            };

            ui.painter().rect_filled(
                Rect::from_min_size(Pos2::new(0.0, 0.0), vec2(ui.available_width() + 80., 210.)),
                0.,
                playlist.image.get_average_color(),
            );

            ui.horizontal(|ui| {
                let image_res =
                    ui.add_sized([144., 144.], image.rounding(5.).sense(Sense::click()));

                if image_res.clicked() {
                    if let Some(image_path) = FileDialog::new().pick_file() {
                        playlist.set_path(image_path.to_string_lossy().to_string());
                        playlist.load_texture(ctx.clone());
                    }
                }

                if image_res.hovered() {
                    ui.painter()
                        .rect_filled(image_res.rect, 5., Color32::from_black_alpha(64));
                }

                image_res.on_hover_cursor(CursorIcon::PointingHand);

                ui.vertical(|ui| {
                    ui.add_space(55.);
                    let name_res = ui.add(
                        Label::new(RichText::new(&playlist.name).strong().size(24.))
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

        if playlist.tracks.is_empty() {
            ui.vertical(|ui| {
                ui.add_space(ui.available_height() / 2. - 20.);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2. - 60.);
                    ui.add(Label::new("Playlist empty!"));
                });
                ui.add_space(10.);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2. - 40.);

                    let settings_res = ui.add(link_label(
                        RichText::new("Library").color(Color32::WHITE),
                        Color32::WHITE,
                    ));
                    if settings_res.clicked() {
                        state.view = View::Library;
                    }
                });
            });
            return;
        }

        ui.add_space(10.);

        let track_num_width = ui.fonts(|fonts| {
            let font_id = ui.style().text_styles[&TextStyle::Body].clone();

            fonts
                .layout_no_wrap(
                    format!("{}.", playlist.tracks.len()),
                    font_id.clone(),
                    Color32::TRANSPARENT,
                )
                .size()
                .x
        });

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
                body.rows(16., playlist.tracks.len(), |mut row| {
                    let index = row.index();
                    let track = &playlist.tracks[index];

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
                        ui.menu_button("Remove from playlist", |ui| {
                            if ui.button("Confirm").clicked() {
                                playlist.tracks.remove(index);
                                self.selection.remove(&index);
                                ui.close_menu();
                            }
                        });

                        if ui.button("Play").clicked() {
                            ui.close_menu();
                            // TODO: Play selected track
                        }
                    });

                    toggle_row_selection(&mut self.selection, index, &response);
                });
            });
    }
}
