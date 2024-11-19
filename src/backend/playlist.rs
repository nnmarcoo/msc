use std::{fs::read_dir, path::Path};

use eframe::egui::{
    include_image, vec2, Context, Grid, ImageButton, Label, ScrollArea, TextWrapMode, Ui,
};

use super::{track::Track, ui::format_seconds};

pub struct Playlist {
    pub tracks: Vec<Track>,
}

impl Playlist {
    pub fn new() -> Self {
        Playlist { tracks: Vec::new() }
    }

    pub fn from_directory(path: &str, ctx: &Context) -> Playlist {
        let mut tracks = Vec::new();
        Self::collect_audio_files(Path::new(path), &mut tracks, ctx);
        Playlist { tracks }
    }

    fn collect_audio_files(dir: &Path, tracks: &mut Vec<Track>, ctx: &Context) {
        if let Ok(entries) = read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(extension) = path.extension() {
                            let ext = extension.to_string_lossy().to_lowercase();
                            if ["mp3", "flac", "m4a", "ogg"].contains(&ext.as_str()) {
                                if let Some(path_str) = path.to_str() {
                                    match Track::new(path_str, ctx) {
                                        track => tracks.push(track),
                                    }
                                }
                            }
                        }
                    } else if path.is_dir() {
                        // Recurse into subdirectory
                        Self::collect_audio_files(&path, tracks, ctx);
                    }
                }
            }
        }
    }

    pub fn to_string(&self) -> String {
        self.tracks
            .iter()
            .map(|track| track.to_string())
            .collect::<Vec<String>>()
            .join("\n\n")
    }

    pub fn display(&self, ui: &mut Ui) {
        let column_width = ui.available_width() / 4.0;
        let row_height = 30.0;

        ScrollArea::vertical().show_rows(ui, row_height, self.tracks.len(), |ui, row_range| {
            Grid::new("playlist")
                .striped(true)
                .min_col_width(column_width)
                .max_col_width(column_width)
                .spacing(vec2(30.0, 30.0))
                .show(ui, |ui| {
                    if row_range.start == 0 {
                        ui.heading("      Title");
                        ui.heading("Artist");
                        ui.heading("Album");
                        ui.heading("Duration");
                        ui.end_row();
                    }

                    for track in &self.tracks[row_range] {
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized(
                                    [20.0, 20.0],
                                    ImageButton::new(include_image!("../../assets/icons/play.png"))
                                        .rounding(3.0),
                                )
                                .clicked()
                            {}
                            ui.add(Label::new(&track.title).wrap_mode(TextWrapMode::Truncate));
                        });

                        ui.add(Label::new(&track.artist).wrap_mode(TextWrapMode::Truncate));
                        ui.add(Label::new(&track.album).wrap_mode(TextWrapMode::Truncate));
                        ui.label(format_seconds(track.duration));
                        ui.end_row();
                    }
                });
        });
    }
}
