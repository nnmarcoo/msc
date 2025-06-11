use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use image::{imageops::FilterType, load_from_memory, open, DynamicImage, ImageResult};

pub trait ImageLoader: Send + Sync {
    fn load_image(&self) -> ImageResult<DynamicImage>;
    fn box_clone(&self) -> Box<dyn ImageLoader>;
}

impl Clone for Box<dyn ImageLoader> {
    fn clone(&self) -> Box<dyn ImageLoader> {
        self.box_clone()
    }
}

#[derive(Clone)]
pub struct FileImage {
    pub path: String,
}

#[derive(Clone)]
pub struct RawImage {
    pub data: Vec<u8>,
}

impl ImageLoader for FileImage {
    fn load_image(&self) -> ImageResult<DynamicImage> {
        open(&self.path)
    }

    fn box_clone(&self) -> Box<dyn ImageLoader> {
        Box::new(self.clone())
    }
}

impl ImageLoader for RawImage {
    fn load_image(&self) -> ImageResult<DynamicImage> {
        load_from_memory(&self.data)
    }

    fn box_clone(&self) -> Box<dyn ImageLoader> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct AsyncImage {
    pub loader: Box<dyn ImageLoader>,
    pub gen_num: Arc<AtomicUsize>,
    pub texture: Arc<Mutex<Option<TextureHandle>>>,
}

impl AsyncImage {
    pub fn new(loader: Box<dyn ImageLoader>) -> Self {
        Self {
            loader,
            gen_num: Arc::new(AtomicUsize::new(0)),
            texture: Arc::new(Mutex::new(None)),
        }
    }

    pub fn load(&self, size: u32, ctx: &Context) {
        let loader = self.loader.clone();
        let texture_arc = Arc::clone(&self.texture);
        let ctx = ctx.clone();

        let gen_num = Arc::clone(&self.gen_num);
        let current_gen = gen_num.fetch_add(1, Ordering::SeqCst) + 1;

        rayon::spawn(move || {
            let img = loader.load_image().unwrap_or_else(|_| {
                load_from_memory(include_bytes!("../../assets/default.png")).unwrap()
            });

            let resized = img.resize(size, size, FilterType::Lanczos3);
            let rgba = resized.to_rgba8();
            let (w, h) = rgba.dimensions();
            let color_image =
                ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba.into_raw());

            if gen_num.load(Ordering::SeqCst) != current_gen {
                return;
            }

            let texture = ctx.load_texture(
                format!("{:?}", Instant::now()), // prob change
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
            loader: Box::new(RawImage {
                data: include_bytes!("../../assets/default.png").to_vec(),
            }),
            gen_num: Arc::new(AtomicUsize::new(0)),
            texture: Arc::new(Mutex::new(None)),
        }
    }
}
