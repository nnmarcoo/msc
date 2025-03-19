use std::cmp::max;

use egui::{scroll_area::ScrollBarVisibility, Context, CursorIcon, Image, ScrollArea, Spinner, Ui};

use crate::core::playlist::Playlist;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct PlayListView {
    playlists: Vec<Playlist>,
    expanded_index: Option<usize>,
}

impl PlayListView {
    pub fn new() -> Self {
        PlayListView {
            playlists: vec![
                // some test playlists // remove later
                Playlist::new(
                    "Pwdadwalaylist 1".to_string(),
                    "Description 1".to_string(),
                    "D:\\spotify\\bass.jpg".to_string(),
                ),
                Playlist::new(
                    "Playlist 2".to_string(),
                    "Description 1".to_string(),
                    "D:\\spotify\\break.png".to_string(),
                ),
                Playlist::new(
                    "Playlist 2.5".to_string(),
                    "Description 1".to_string(),
                    "D:\\spotify\\brother.jpg".to_string(),
                ),
                Playlist::new(
                    "Playlist 3".to_string(),
                    "Description 1".to_string(),
                    "D:\\spotify\\chillaxin.png".to_string(),
                ),
                Playlist::new(
                    "Playlist 4".to_string(),
                    "Description 1".to_string(),
                    "D:\\spotify\\drwyd.png".to_string(),
                ),
                Playlist::new(
                    "Playlist 5".to_string(),
                    "Description 1".to_string(),
                    "D:\\spotify\\no.jpg".to_string(),
                ),
                Playlist::new(
                    "Playlist 6".to_string(),
                    "Description 1".to_string(),
                    "D:\\spotify\\over.png".to_string(),
                ),
                Playlist::new(
                    "Playlist 7".to_string(),
                    "Description 1".to_string(),
                    "D:\\spotify\\ppur.jpg".to_string(),
                ),
                Playlist::new(
                    "Playlist 8".to_string(),
                    "Description 1".to_string(),
                    "D:\\spotify\\punk.png".to_string(),
                ),
                Playlist::new(
                    "Playlist 9".to_string(),
                    "Description 1".to_string(),
                    "D:\\spotify\\vamp.jpg".to_string(),
                ),
                Playlist::new(
                    "Playlist 10".to_string(),
                    "Description 1".to_string(),
                    "D:\\spotify\\xtreem hy.jpg".to_string(),
                ),
                Playlist::new(
                    "Playlist 11".to_string(),
                    "Description 1".to_string(),
                    "D:\\spotify\\zooom.png".to_string(),
                ),
                Playlist::new(
                    "Playlist 12".to_string(),
                    "Description 1".to_string(),
                    "D:\\spotify\\debug.jpg".to_string(),
                ),
            ],
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
                            ScrollArea::vertical()
                                .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                                .show(ui, |ui| {
                                    ui.heading(&expanded_playlist.name);
                                    ui.label("Tracks:");
                                    for track in &expanded_playlist.tracks {
                                        ui.label(track);
                                    }
                                });
                        }
                    }
                }
            });
        });
    }
}
