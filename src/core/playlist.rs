use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use blake3::Hash;
use color_thief::{get_palette, ColorFormat};
use egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions};
use image::{imageops::FilterType, load_from_memory, DynamicImage};
use serde::{Deserialize, Serialize};

// THIS SUCKS

#[derive(Serialize, Deserialize)]
pub struct Playlist {
    pub name: String,
    pub description: String,
    pub image_path: String,
    pub prev_size: f32,
    pub gen_num: Arc<AtomicUsize>,
    pub tracks: Vec<Hash>,
    #[serde(skip)]
    pub texture: Arc<Mutex<Option<TextureHandle>>>,
    #[serde(skip)]
    average_color: Arc<Mutex<Color32>>,
}

impl Playlist {
    pub fn new(name: String, description: String, image_path: String) -> Self {
        Playlist {
            name,
            description,
            image_path,
            texture: Arc::new(Mutex::new(None)),
            prev_size: 0.,
            gen_num: Arc::new(AtomicUsize::new(0)),
            tracks: vec![],
            average_color: Arc::new(Mutex::new(Color32::BLACK)),
        }
    }

    pub fn load_image(&self, size: u32, ctx: &Context) {
        let image_path = self.image_path.clone();
        let texture_arc = Arc::clone(&self.texture);
        let ctx = ctx.clone();

        let gen_num = Arc::clone(&self.gen_num);
        let current_gen = gen_num.fetch_add(1, Ordering::SeqCst) + 1;

        let average_color = Arc::clone(&self.average_color);

        rayon::spawn(move || {
            let img = image::open(&image_path).unwrap_or_else(|_| {
                load_from_memory(include_bytes!("../../assets/default.png"))
                    .expect("Failed to load default image")
            });

            let resized = img.resize(size, size, FilterType::Lanczos3);
            let rgba = resized.to_rgba8();
            let (w, h) = rgba.dimensions();
            let color_image =
                ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba.into_raw());

            if gen_num.load(Ordering::SeqCst) != current_gen {
                return;
            }

            let texture = ctx.load_texture(&image_path, color_image, TextureOptions::NEAREST);
            *texture_arc.lock().unwrap() = Some(texture);

            *average_color.lock().unwrap() = dominant_color(&img);
        });
    }

    pub fn texture_or_load(&mut self, size: f32, ctx: &Context) -> Option<TextureHandle> {
        if self.prev_size != size {
            self.prev_size = size;
            self.load_image(size as u32, ctx);
        }

        if let Some(texture) = self.get_texture_handle() {
            return Some(texture);
        }

        None
    }

    pub fn get_texture_handle(&self) -> Option<TextureHandle> {
        self.texture.lock().unwrap().clone()
    }

    pub fn get_average_color(&self) -> Color32 {
        self.average_color.lock().unwrap().clone()
    }
}

fn dominant_color(image: &DynamicImage) -> Color32 {
    let rgb_image = image.to_rgb8();
    let pixels = rgb_image.as_raw();

    if let Ok(palette) = get_palette(pixels, ColorFormat::Rgb, 10, 2) {
        if let Some(color) = palette.first() {
            return Color32::from_rgb(color.r, color.g, color.b);
        }
    }
    Color32::BLACK
}
