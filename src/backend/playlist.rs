use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use eframe::egui::{Context, TextureHandle};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use super::{image_display::ImageDisplay, track::Track};

#[derive(Serialize, Deserialize)]
pub struct Playlist {
    pub tracks: Vec<Track>,
    pub name: String,
    pub desc: String,
    #[serde(skip)]
    image: ImageDisplay,
    image_path: String,
}

impl Playlist {
    pub fn new() -> Self {
        Playlist {
            tracks: Vec::new(),
            name: String::from("My Playlist"),
            desc: String::new(),
            image: ImageDisplay::new(),
            image_path: String::new(),
        }
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn from_directory(path: &str) -> Playlist {
        let tracks = Self::collect_audio_files(Path::new(path));
        Playlist {
            tracks,
            name: String::new(),
            desc: String::new(),
            image: ImageDisplay::new(),
            image_path: String::new(),
        }
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
                            if ["mp3", "flac", "m4a", "wav", "ogg"].contains(&ext.as_str()) {
                                if let Some(path_str) = path.to_str() {
                                    if let Some(track) = Track::new(path_str) {
                                        return vec![track].into_par_iter();
                                    }
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

    pub fn get_texture(&self) -> Option<TextureHandle> {
        self.image.get_texture()
    }

    pub fn load_texture(&mut self, ctx: Context) {
        self.image.load_texture(self.image_path.clone(), ctx);
    }

    pub fn set_path(&mut self, path: String) {
        self.image_path = path;
        self.image.clear_texture();
    }
}
