use std::cmp::max;

use egui::{
    scroll_area::ScrollBarVisibility, vec2, Context, CursorIcon, Image, Rect, ScrollArea, Spinner,
    Ui,
};

use crate::core::playlist::Playlist;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct PlayListView {
    playlists: Vec<Playlist>,
    expanded_index: Option<usize>,
}

impl PlayListView {
    pub fn new() -> Self {
        PlayListView {
            playlists: vec![],
            expanded_index: None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, ctx: &Context) {
        let available_width = ui.available_width();
        let zoom = ctx.zoom_factor();

        let base_gap = 3.;
        let base_min_image_size = 125.;
        let gap = base_gap * zoom;
        let min_image_size = base_min_image_size * zoom;

        let num_columns = max(
            1,
            ((available_width + gap) / (min_image_size + gap)).floor() as usize,
        );

        let size = (available_width - (num_columns as f32 - 1.) * gap) / num_columns as f32;
        let scaled_size = size * zoom;
        let image_vec = egui::vec2(size, size);

        ui.spacing_mut().item_spacing = egui::vec2(gap, gap);

        let expanded_row = self.expanded_index.map(|i| i / num_columns);

        ScrollArea::vertical().show(ui, |ui| {
            ui.vertical(|ui| {
                let n = self.playlists.len();
                let num_rows = (n + num_columns - 1) / num_columns;

                for row in 0..num_rows {
                    ui.horizontal(|ui| {
                        for col in 0..num_columns {
                            let i = row * num_columns + col;
                            if i < n {
                                if let Some(texture) =
                                    self.playlists[i].texture_or_load(scaled_size, ctx)
                                {
                                    let res = ui.add(
                                        Image::new(&texture)
                                            .fit_to_exact_size(image_vec)
                                            .sense(egui::Sense::click()),
                                    );

                                    if res.clicked() {
                                        self.expanded_index = if Some(i) == self.expanded_index {
                                            None
                                        } else {
                                            Some(i)
                                        };
                                    }

                                    res.on_hover_cursor(CursorIcon::PointingHand);
                                } else {
                                    ui.add_sized(image_vec, Spinner::new());
                                }
                            }
                        }
                    });

                    if Some(row) == expanded_row {
                        if let Some(expanded_idx) = self.expanded_index {
                            let expanded_playlist = &self.playlists[expanded_idx];

                            let width = size * 2.;
                            let start_pos = ui.cursor().min;

                            ui.painter().rect_filled(
                                Rect::from_min_size(start_pos, vec2(ui.available_width(), width)),
                                0.,
                                expanded_playlist.get_average_color(),
                            );

                            ui.add_space(gap);
                            ScrollArea::vertical()
                                .max_height(width)
                                .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                                .show(ui, |ui| {
                                    ui.heading(&expanded_playlist.name);
                                    ui.label("Tracks:");
                                    for track in &expanded_playlist.tracks {
                                        ui.label(track);
                                    }
                                });
                            ui.add_space(width - (ui.cursor().min.y - start_pos.y));
                        }
                    }
                }
            });
        });
    }
}
