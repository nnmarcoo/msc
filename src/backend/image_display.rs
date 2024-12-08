use std::{
    io::Cursor,
    path::Path,
    sync::{Arc, Mutex},
};

use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};
use image::{imageops::FilterType, load_from_memory, open, DynamicImage, ImageFormat};
use lofty::{file::TaggedFileExt, probe::Probe};

use crate::constants::DEFAULT_IMAGE_BYTES;

#[derive(Default)]
pub struct ImageDisplay {
    texture: Arc<Mutex<Option<TextureHandle>>>,
    pub loaded: bool,
}

impl ImageDisplay {
    pub fn new() -> Self {
        ImageDisplay {
            loaded: false,
            texture: Arc::new(Mutex::new(None)),
        }
    }

    pub fn load_texture(&mut self, path: String, ctx: Context) {
        if self.loaded {
            return;
        }
        self.loaded = true;

        let texture_handle = Arc::clone(&self.texture);

        rayon::spawn(move || {
            let image = if let Ok(image) = open(&Path::new(&path)) {
                load_image(image.resize_exact(40, 40, FilterType::Lanczos3))
            } else {
                match Probe::open(path.clone()) {
                    Ok(probe) => match probe.read() {
                        Ok(tagged_file) => {
                            if let Some(picture) = tagged_file
                                .primary_tag()
                                .and_then(|tag| tag.pictures().first())
                            {
                                let image_data = picture.data();
                                if let Ok(loaded_image) = load_from_memory(image_data) {
                                    load_image(loaded_image)
                                } else {
                                    default_image()
                                }
                            } else {
                                default_image()
                            }
                        }
                        Err(_) => default_image(),
                    },
                    Err(_) => default_image(),
                }
            };

            let texture = ctx.load_texture("track_texture", image, TextureOptions::NEAREST);

            *texture_handle.lock().unwrap() = Some(texture);
        });
    }

    pub fn get_texture(&self) -> Option<TextureHandle> {
        self.texture.lock().ok()?.clone()
    }

    pub fn clear_texture(&mut self) {
        self.texture = Arc::new(Mutex::new(None));
        self.loaded = false;
    }
}

fn default_image() -> ColorImage {
    let img = image::load(Cursor::new(DEFAULT_IMAGE_BYTES), ImageFormat::Png).unwrap();

    let img = img.to_rgba8();
    let (width, height) = img.dimensions();

    ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &img)
}

fn load_image(image: DynamicImage) -> ColorImage {
    let rgba_image = image.to_rgba8();
    let dimensions = rgba_image.dimensions();
    let pixels = rgba_image.into_raw();

    ColorImage::from_rgba_unmultiplied([dimensions.0 as usize, dimensions.1 as usize], &pixels)
}
