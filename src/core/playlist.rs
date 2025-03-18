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
    pub prev_size: f32,
}

impl Playlist {
    pub fn new(name: String, description: String, image_path: String) -> Self {
        Playlist {
            name,
            description,
            image_path,
            texture: Arc::new(Mutex::new(None)),
            prev_size: 0.,
        }
    }

    pub fn load_image(&self, size: u32, ctx: &Context) {
        let image_path = self.image_path.clone();
        let texture_arc = Arc::clone(&self.texture);
        let ctx = ctx.clone();

        rayon::spawn(move || {
            let img = image::open(&image_path).unwrap_or_else(|err| {
                eprintln!(
                    "Failed to open image at {}: {}. Creating an empty fallback image.",
                    image_path, err
                );
                DynamicImage::new_rgba8(size, size)
            });

            let resized = img.resize_exact(size, size, FilterType::Lanczos3);
            let rgba = resized.to_rgba8();
            let (w, h) = rgba.dimensions();
            let color_image =
                ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba.into_raw());

            let texture = ctx.load_texture(&image_path, color_image, TextureOptions::NEAREST);
            *texture_arc.lock().unwrap() = Some(texture);
        });
    }

    pub fn display_or_load(&mut self, zoom_scale: f32, size: f32, ui: &mut Ui) {
        if self.prev_size != size {
            self.prev_size = size;
            self.load_image((size * zoom_scale) as u32, ui.ctx());
        } else if let Some(texture) = self.get_texture_handle() {
            
            ui.add(Image::new(&texture).fit_to_exact_size(vec2(size, size)));
        }
    }

    pub fn get_texture_handle(&self) -> Option<TextureHandle> {
        self.texture.lock().unwrap().clone()
    }
}
