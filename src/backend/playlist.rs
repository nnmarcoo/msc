use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::constants::DEFAULT_IMAGE_BYTES;

use super::{image::serial_image::SerialImage, track::Track};

#[derive(Serialize, Deserialize)]
pub struct Playlist {
    pub tracks: Vec<Track>,
    pub name: String,
    pub image: SerialImage,
    #[serde(skip)]
    pub texture: Option<TextureHandle>,
}

impl Playlist {
    pub fn new() -> Self {
        Playlist {
            tracks: Vec::new(),
            name: String::from("New Playlist"),
            image: Self::default_image(),
            texture: None,
        }
    }

    pub fn from_directory(path: &str) -> Playlist {
        let tracks = Self::collect_audio_files(Path::new(path));
        Playlist {
            tracks,
            name: String::from(""),
            image: Self::default_image(),
            texture: None,
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

    fn default_image() -> SerialImage {
        let image = image::load_from_memory(DEFAULT_IMAGE_BYTES)
            .expect("Failed to load default image")
            .to_rgba8();

        let size = [image.width() as usize, image.height() as usize];
        let pixels = image
            .pixels()
            .map(|p| {
                let [r, g, b, a] = p.0;
                ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
            })
            .collect();

        SerialImage { size, pixels }
    }

    pub fn load_texture(&mut self, ctx: &Context) {
        if self.texture.is_none() {
            let color_image: ColorImage = self.image.clone().into();
            let texture =
                ctx.load_texture("playlist_texture", color_image, TextureOptions::default());
            self.texture = Some(texture);
        }
    }
}
