use eframe::egui::{include_image, vec2, Grid, ImageButton, Ui};

use super::{track::Track, ui::format_seconds};

pub struct Playlist {
    pub tracks: Vec<Track>,
}

impl Playlist {
    pub fn new() -> Self {
        Playlist { tracks: Vec::new() }
    }

    pub fn to_string(&self) -> String {
        self.tracks
            .iter()
            .map(|track| track.to_string())
            .collect::<Vec<String>>()
            .join("\n\n")
    }

    pub fn display(&self, ui: &mut Ui) {
        ui.allocate_ui(ui.available_size(), |ui| {
            Grid::new("playlist")
                .striped(true)
                .min_col_width(ui.available_width() / 4.)
                .spacing(vec2(0., 16.))
                .show(ui, |ui| {
                    ui.heading("      Title");
                    ui.heading("Artist");
                    ui.heading("Album");
                    ui.heading("Duration");
                    ui.end_row();

                    for track in &self.tracks {
                        ui.horizontal(|ui| {
                            if ui
                                    .add_sized([16., 16.], ImageButton::new(include_image!("../../assets/icons/play.png")).rounding(3.))
                                    .clicked()
                                {

                                 }
                            ui.label(&track.title);
                        });
                        
                        ui.label(&track.artist);
                        ui.label(&track.album);
                        ui.label(format_seconds(track.duration));
                        ui.end_row();
                    }
                });
        });
    }
}
