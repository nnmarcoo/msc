use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use std::sync::atomic::{AtomicUsize, Ordering};

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use image::{imageops::FilterType, load_from_memory};

#[derive(Clone)]
pub struct AsyncImage {
    pub data: Arc<Vec<u8>>,
    pub gen_num: Arc<AtomicUsize>,
    pub texture: Arc<Mutex<Option<TextureHandle>>>,
}

impl AsyncImage {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: Arc::new(data),
            gen_num: Arc::new(AtomicUsize::new(0)),
            texture: Arc::new(Mutex::new(None)),
        }
    }

    pub fn load(&self, size: u32, ctx: &Context) {
        let data = Arc::clone(&self.data);
        let texture_arc = Arc::clone(&self.texture);
        let ctx = ctx.clone();
        let gen_num = Arc::clone(&self.gen_num);
        let current_gen = gen_num.fetch_add(1, Ordering::SeqCst) + 1;

        rayon::spawn(move || {
            let Ok(img) = load_from_memory(&data) else {
                return;
            };

            let resized = img.resize(size, size, FilterType::Lanczos3);
            let rgba = resized.to_rgba8();
            let (w, h) = rgba.dimensions();
            let color_image =
                ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba.into_raw());

            if gen_num.load(Ordering::SeqCst) != current_gen {
                return;
            }

            let texture = ctx.load_texture(
                format!("{:?}", Instant::now()),
                color_image,
                TextureOptions::NEAREST,
            );

            *texture_arc.lock().unwrap() = Some(texture);
        });
    }

    pub fn get_texture_handle(&self) -> Option<TextureHandle> {
        self.texture.lock().unwrap().clone()
    }
}

impl Default for AsyncImage {
    fn default() -> Self {
        Self {
            data: Arc::new(Vec::new()),
            gen_num: Arc::new(AtomicUsize::new(0)),
            texture: Arc::new(Mutex::new(None)),
        }
    }
}
