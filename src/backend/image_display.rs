use std::{
    io::Cursor,
    path::Path,
    sync::{Arc, Mutex},
};

use eframe::egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions};
use image::{imageops::FilterType, load_from_memory, open, DynamicImage, ImageFormat};
use lofty::{file::TaggedFileExt, probe::Probe};

use crate::constants::DEFAULT_IMAGE_BYTES;

#[derive(Default)]
pub struct ImageDisplay {
    texture_small: Arc<Mutex<Option<TextureHandle>>>,
    texture_medium: Arc<Mutex<Option<TextureHandle>>>,
    texture_large: Arc<Mutex<Option<TextureHandle>>>,
    average_color: Arc<Mutex<Color32>>,
    pub loaded: bool,
}

impl ImageDisplay {
    pub fn new() -> Self {
        ImageDisplay {
            loaded: false,
            texture_small: Arc::new(Mutex::new(None)),
            texture_medium: Arc::new(Mutex::new(None)),
            texture_large: Arc::new(Mutex::new(None)),
            average_color: Arc::new(Mutex::new(Color32::BLACK)),
        }
    }

    pub fn load_texture(&mut self, path: String, ctx: Context) {
        if self.loaded {
            return;
        }
        self.loaded = true;

        let texture_small = Arc::clone(&self.texture_small);
        let texture_medium = Arc::clone(&self.texture_medium);
        let texture_large = Arc::clone(&self.texture_large);
        let average_color = Arc::clone(&self.average_color);

        let ctx = Arc::new(ctx);
        let path_clone = path.clone();

        rayon::spawn({
            let ctx = Arc::clone(&ctx);
            move || {
                let image = open(&Path::new(&path)).unwrap_or_else(|_| {
                    Probe::open(path_clone.clone())
                        .ok()
                        .and_then(|probe| probe.read().ok())
                        .and_then(|tagged_file| {
                            tagged_file
                                .primary_tag()
                                .and_then(|tag| tag.pictures().first())
                                .and_then(|picture| load_from_memory(picture.data()).ok())
                        })
                        .unwrap_or_else(default_image)
                });

                for (s, texture_arc) in [
                    (48, &texture_small),
                    (144, &texture_medium),
                    (192, &texture_large),
                ] {
                    let ctx = Arc::clone(&ctx);
                    let image = image.clone();
                    let imag2e = image.clone();
                    let texture_arc = Arc::clone(texture_arc);

                    rayon::spawn(move || {
                        let texture = ctx.load_texture(
                            "track_texture",
                            load_image(image, s, s),
                            TextureOptions::NEAREST,
                        );
                        *texture_arc.lock().unwrap() = Some(texture);
                    });

                    // wasteful
                    let color = calculate_average(&imag2e);
                    *average_color.lock().unwrap() = color;
                }
            }
        });
    }

    pub fn get_texture_small(&self) -> Option<TextureHandle> {
        self.texture_small.lock().ok()?.clone()
    }

    pub fn get_texture_medium(&self) -> Option<TextureHandle> {
        self.texture_medium.lock().ok()?.clone()
    }

    pub fn get_texture_large(&self) -> Option<TextureHandle> {
        self.texture_large.lock().ok()?.clone()
    }

    pub fn get_average_color(&self) -> Color32 {
        self.average_color.lock().unwrap().clone()
    }

    pub fn clear_texture(&mut self) {
        self.texture_small = Arc::new(Mutex::new(None));
        self.texture_medium = Arc::new(Mutex::new(None));
        self.texture_large = Arc::new(Mutex::new(None));
        self.loaded = false;
    }
}

fn default_image() -> DynamicImage {
    image::load(Cursor::new(DEFAULT_IMAGE_BYTES), ImageFormat::Png).unwrap()
}

fn load_image(image: DynamicImage, width: u32, height: u32) -> ColorImage {
    let rgba_image = image
        .resize_exact(width, height, FilterType::Lanczos3)
        .to_rgba8();
    let dimensions = rgba_image.dimensions();
    let pixels = rgba_image.into_raw();

    ColorImage::from_rgba_unmultiplied([dimensions.0 as usize, dimensions.1 as usize], &pixels)
}

fn calculate_average(image: &DynamicImage) -> Color32 {
    let img = image.to_rgba8();

    let (width, height) = img.dimensions();
    let mut total_r = 0u64;
    let mut total_g = 0u64;
    let mut total_b = 0u64;
    let mut total_a = 0u64;

    img.pixels().for_each(|pixel| {
        total_r += pixel[0] as u64;
        total_g += pixel[1] as u64;
        total_b += pixel[2] as u64;
        total_a += pixel[3] as u64;
    });

    let pixel_count = (width * height) as u64;
    let avg_r = (total_r / pixel_count) as u8;
    let avg_g = (total_g / pixel_count) as u8;
    let avg_b = (total_b / pixel_count) as u8;
    let avg_a = (total_a / pixel_count) as u8;

    Color32::from_rgba_unmultiplied(avg_r, avg_g, avg_b, avg_a)
}
