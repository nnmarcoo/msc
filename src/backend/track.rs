use std::{io::Cursor, sync::Arc};

use eframe::{
    egui::{mutex::RwLock, ColorImage, Context, TextureHandle, TextureId, TextureOptions},
    epaint::TextureManager,
};

use image::{imageops::FilterType, load_from_memory};
use lofty::{
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::ItemKey,
};

pub struct Track {
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub image: Option<TextureHandle>,
    pub duration: f32,
}

impl Track {
    pub fn new(path: &str, ctx: &Context) -> Self {
        let tagged_file = Probe::open(path).unwrap().read().unwrap();
        let properties = tagged_file.properties();
        let tag = tagged_file.primary_tag();

        let title = tag
            .and_then(|t| t.get_string(&ItemKey::TrackTitle).map(String::from))
            .unwrap_or("NA".to_string());
        let artist = tag
            .and_then(|t| t.get_string(&ItemKey::AlbumArtist).map(String::from))
            .unwrap_or("NA".to_string());

        let duration = properties.duration().as_secs_f32();

        let image = if let Some(picture) = tagged_file
            .primary_tag()
            .and_then(|tag| tag.pictures().first())
        {
            let image_data = picture.data();
            let img = load_from_memory(image_data).ok().unwrap().resize_exact(
                48,
                48,
                FilterType::Lanczos3,
            );

            let rgba_img = img.to_rgba8();
            let size = [rgba_img.width() as usize, rgba_img.height() as usize];
            let pixels = rgba_img.into_raw();

            Some(ctx.load_texture(
                title.clone(),
                ColorImage::from_rgba_unmultiplied(size, &pixels),
                TextureOptions::LINEAR,
            ))
        } else {
            let img = image::load(
                Cursor::new(include_bytes!("../../assets/icons/defaultborder.png")),
                image::ImageFormat::Png,
            )
            .unwrap();

            let img = img.to_rgba8();
            let (width, height) = img.dimensions();

            Some(ctx.load_texture(
                "default_image",
                ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &img),
                TextureOptions::LINEAR,
            ))
        };

        Track {
            file_path: path.to_string(),
            title,
            artist,
            image,
            duration,
        }
    }

    pub fn default() -> Self {
        Track {
            file_path: String::from("-"),
            title: String::new(),
            artist: String::new(),
            image: None,
            duration: -1.,
        }
    }
}
