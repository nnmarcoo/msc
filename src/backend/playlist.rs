use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use eframe::egui::{include_image, vec2, Grid, ImageButton, Label, ScrollArea, TextWrapMode, Ui};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use super::{track::Track, ui::format_seconds};

pub struct Playlist {
    pub tracks: Vec<Track>,
}

impl Playlist {
    pub fn new() -> Self {
        Playlist { tracks: Vec::new() }
    }

    pub fn from_directory(path: &str) -> Playlist {
        let tracks = Self::collect_audio_files(Path::new(path));
        Playlist { tracks }
    }

    fn collect_audio_files(dir: &Path) -> Vec<Track> {
        if let Ok(entries) = read_dir(dir) {
            let entries: Vec<PathBuf> = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect();

            entries
                .par_iter()
                .flat_map(|path| {
                    if path.is_file() {
                        if let Some(extension) = path.extension() {
                            let ext = extension.to_string_lossy().to_lowercase();
                            if ["mp3", "flac", "m4a", "ogg"].contains(&ext.as_str()) {
                                if let Some(path_str) = path.to_str() {
                                    return vec![Track::new(path_str)].into_par_iter();
                                }
                            }
                        }
                    } else if path.is_dir() {
                        return Self::collect_audio_files(path).into_par_iter();
                    }
                    Vec::new().into_par_iter()
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn display(&self, ui: &mut Ui) {
        let column_width = ui.available_width() / 4.;
        let row_height = 40.;

        ScrollArea::vertical().show(ui, |ui| {
            Grid::new("playlist")
                .striped(true)
                .min_col_width(column_width)
                .max_col_width(column_width)
                .spacing(vec2(30., 30.))
                .show(ui, |ui| {
                    ui.heading("      Title");
                    ui.heading("Artist");
                    ui.heading("Album");
                    ui.heading("Duration");
                    ui.end_row();

                    for track in &self.tracks {
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
