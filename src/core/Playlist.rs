use std::sync::{Arc, Mutex};

use egui::{ColorImage, Context, TextureHandle};
use image::imageops::FilterType;
use serde::{Deserialize, Serialize};

// TODO: pull texture out of this and make anew struct called ImageDisplay that has functions to load a new size, get the texture handle. it stores the path or data.

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
}
