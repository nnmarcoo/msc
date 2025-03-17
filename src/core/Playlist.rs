use std::sync::{Arc, Mutex};

use egui::{vec2, ColorImage, Context, Image, TextureHandle, TextureOptions, Ui};
use image::{imageops::FilterType, DynamicImage};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Playlist {
    pub name: String,
    pub description: String,
    pub image_path: String,
    #[serde(skip)]
    pub texture: Arc<Mutex<Option<TextureHandle>>>,
}

impl Playlist {
    pub fn new(name: String, description: String, image_path: String) -> Self {
        Playlist {
            name,
            description,
            image_path,
            texture: Arc::new(Mutex::new(None)),
        }
    }

    pub fn load_image(&self, width: u32, height: u32, ctx: &Context) {
        let image_path = self.image_path.clone();
        let texture_arc = Arc::clone(&self.texture);
        let ctx = ctx.clone();

        rayon::spawn(move || {
            let img = image::open(&image_path).unwrap_or_else(|err| {
                eprintln!(
                    "Failed to open image at {}: {}. Creating an empty fallback image.",
                    image_path, err
                );
                DynamicImage::new_rgba8(width, height)
            });

            let resized = img.resize_exact(width, height, FilterType::Lanczos3);
            let rgba = resized.to_rgba8();
            let (w, h) = rgba.dimensions();
            let color_image =
                ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba.into_raw());

            let texture = ctx.load_texture(&image_path, color_image, TextureOptions::NEAREST);
            *texture_arc.lock().unwrap() = Some(texture);
        });
    }

    pub fn display(&self, width: f32, height: f32, ui: &mut Ui) {
        if let Some(texture) = self.get_texture_handle() {
            ui.add(Image::new(&texture).fit_to_exact_size(vec2(width, height)));
        }
    }

    pub fn get_texture_handle(&self) -> Option<TextureHandle> {
        self.texture.lock().unwrap().clone()
    }
}
