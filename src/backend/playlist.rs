use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};
use image::load_from_memory;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::constants::DEFAULT_IMAGE_BYTES;

use super::track::Track;

#[derive(Serialize, Deserialize, Clone)]
pub struct Playlist {
    pub tracks: Vec<Track>,
    pub name: String,
    pub image_path: String,
    #[serde(skip)]
    pub texture: Option<TextureHandle>,
}

impl Playlist {
    pub fn new() -> Self {
        Playlist {
            tracks: Vec::new(),
            name: String::from("New Playlist"),
            image_path: String::new(),
            texture: None,
        }
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn from_directory(path: &str) -> Playlist {
        let tracks = Self::collect_audio_files(Path::new(path));
        Playlist {
            tracks,
            name: String::from(""),
            image_path: String::new(),
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

    pub fn load_texture(&mut self, ctx: &Context) {
        if self.texture.is_none() {
            if let Ok(img) = image::open(&Path::new(&self.image_path)) {
                let color_image: ColorImage = {
                    let rgba_image = img.to_rgba8();
                    let dimensions = rgba_image.dimensions();
                    let pixels = rgba_image.into_raw();

                    let color_image = ColorImage::from_rgba_unmultiplied(
                        [dimensions.0 as usize, dimensions.1 as usize],
                        &pixels,
                    );
                    color_image
                };

                let texture =
                    ctx.load_texture("playlist_texture", color_image, TextureOptions::default());
                self.texture = Some(texture);
            } else {
                // duplicate code in track.rs
                let img = load_from_memory(DEFAULT_IMAGE_BYTES).unwrap();
                let rgba_image = img.to_rgba8();
                let dimensions = rgba_image.dimensions();
                let pixels = rgba_image.into_raw();

                let color_image = ColorImage::from_rgba_unmultiplied(
                    [dimensions.0 as usize, dimensions.1 as usize],
                    &pixels,
                );

                self.texture = Some(ctx.load_texture(
                    "default_playlist_texture",
                    color_image,
                    TextureOptions::default(),
                ));
            }
        }
    }
}
