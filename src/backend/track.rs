use std::{io::Cursor, path::Path};

use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};

use image::{imageops::FilterType, load_from_memory};
use lofty::{
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::ItemKey,
};
use serde::{Deserialize, Serialize};

use std::sync::{Arc, Mutex};

use crate::constants::DEFAULT_IMAGE_BORDER_BYTES;

use super::image::image_loader::ImageLoader;

#[derive(Serialize, Deserialize, Clone)]
pub struct Track {
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f32,
    #[serde(skip)]
    pub texture: Arc<Mutex<Option<TextureHandle>>>,
    #[serde(skip)]
    pub loading: bool,
}

impl Default for Track {
    fn default() -> Self {
        Track {
            file_path: String::new(),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            duration: 0.,
            texture: Arc::new(Mutex::new(None)),
            loading: false,
        }
    }
}

impl Track {
    pub fn new(path: &str) -> Option<Self> {
        if let Ok(tagged_file) = Probe::open(path).unwrap().read() {
            let properties = tagged_file.properties();
            let tag = tagged_file.primary_tag();
            let file_name = Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let title = tag
                .and_then(|t| t.get_string(&ItemKey::TrackTitle).map(String::from))
                .unwrap_or(file_name.clone());

            let artist = tag
                .and_then(|t| t.get_string(&ItemKey::AlbumArtist).map(String::from))
                .unwrap_or(String::new());

            let album = tag
                .and_then(|t| t.get_string(&ItemKey::AlbumTitle).map(String::from))
                .unwrap_or(String::new());

            let duration = properties.duration().as_secs_f32();

            Some(Track {
                file_path: path.to_string(),
                title,
                artist,
                album,
                duration,
                texture: Arc::new(Mutex::new(None)),
                loading: false,
            })
        } else {
            None
        }
    }

    pub fn load_texture_async(&mut self, ctx: Context, loader: Arc<ImageLoader>) {
        if !self.texture.lock().unwrap().is_none() || self.loading {
            return;
        }

        self.loading = true;

        let file_path = self.file_path.clone();
        let texture_handle = Arc::clone(&self.texture);

        loader.queue_image_load(move || {
            let tagged_file = Probe::open(&file_path).unwrap().read().unwrap();

            let image = if let Some(picture) = tagged_file
                .primary_tag()
                .and_then(|tag| tag.pictures().first())
            {
                let image_data = picture.data();

                // duplicate code in playlist.rs
                let img =
                    load_from_memory(image_data)
                        .ok()
                        .unwrap()
                        .resize(48, 48, FilterType::Lanczos3);

                let rgba_img = img.to_rgba8();
                let size = [rgba_img.width() as usize, rgba_img.height() as usize];
                let pixels = rgba_img.into_raw();

                ColorImage::from_rgba_unmultiplied(size, &pixels)
            } else {
                let img = image::load(
                    Cursor::new(DEFAULT_IMAGE_BORDER_BYTES),
                    image::ImageFormat::Png,
                )
                .unwrap();

                let img = img.to_rgba8();
                let (width, height) = img.dimensions();

                ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &img)
            };

            let texture = ctx.load_texture("track_texture", image, TextureOptions::default());

            *texture_handle.lock().unwrap() = Some(texture);
        });
    }

    pub fn get_texture(&self) -> Option<TextureHandle> {
        self.texture.lock().ok()?.clone()
    }
}
