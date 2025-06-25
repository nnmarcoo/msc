use std::{borrow::Cow, path::Path};

use blake3::{Hash, Hasher};
use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use image::load_from_memory;
use lofty::{
    file::{AudioFile, TaggedFileExt},
    picture::PictureType,
    probe::Probe,
    tag::Accessor,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Track {
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub duration: f32,
    pub image_data: Vec<u8>,
    pub hash: Hash,
    #[serde(skip)]
    pub texture: Option<TextureHandle>,
}

impl Track {
    pub fn default() -> Self {
        Self {
            file_path: String::new(),
            title: "Title".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            genre: "Genre".to_string(),
            duration: 0.0,
            image_data: include_bytes!("../../assets/default.png").to_vec(),
            hash: blake3::hash(&[]),
            texture: None,
        }
    }

    pub fn new(path: &str) -> Option<Self> {
        // Hash file using memory-mapped I/O
        let mut hasher = Hasher::new();
        let file_hash = hasher.update_mmap(Path::new(path)).ok()?.finalize();

        let tagged_file = Probe::open(path).ok()?.read().ok()?;
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag());

        let properties = tagged_file.properties();
        let duration = properties.duration().as_secs_f32();

        let fallback_title = Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Unknown");

        let (title, artist, album, genre, image_data): (String, String, String, String, Vec<u8>) =
            if let Some(tag) = tag {
                let title = tag
                    .title()
                    .unwrap_or(Cow::Borrowed(fallback_title))
                    .to_string();
                let artist = tag.artist().unwrap_or(Cow::Borrowed("Unknown")).to_string();
                let album = tag.album().unwrap_or(Cow::Borrowed("Unknown")).to_string();
                let genre = tag.genre().unwrap_or(Cow::Borrowed("Unknown")).to_string();

                let image_data = tag
                    .pictures()
                    .iter()
                    .find(|pic| pic.pic_type() == PictureType::CoverFront)
                    .or_else(|| tag.pictures().first())
                    .map(|pic| pic.data().to_vec())
                    .unwrap_or_else(|| include_bytes!("../../assets/default.png").to_vec());

                (title, artist, album, genre, image_data)
            } else {
                (
                    fallback_title.to_string(),
                    "Unknown".to_string(),
                    "Unknown".to_string(),
                    "Unknown".to_string(),
                    include_bytes!("../../assets/default.png").to_vec(),
                )
            };

        Some(Self {
            file_path: path.to_string(),
            title,
            artist,
            album,
            genre,
            duration,
            image_data,
            hash: file_hash,
            texture: None,
        })
    }

    pub fn load_texture(&mut self, ctx: &Context) -> Option<&TextureHandle> {
        if self.texture.is_none() {
            let img = load_from_memory(&self.image_data).ok()?.to_rgba8();
            let (w, h) = img.dimensions();
            let color_image =
                ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &img.into_raw());

            let texture =
                ctx.load_texture(self.title.clone(), color_image, TextureOptions::NEAREST);

            self.texture = Some(texture);
        }

        self.texture.as_ref()
    }
}
