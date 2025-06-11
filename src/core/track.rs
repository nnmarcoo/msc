use std::{borrow::Cow, path::Path};

use lofty::{
    file::{AudioFile, TaggedFileExt},
    picture::PictureType,
    probe::Probe,
    tag::Accessor,
};
use serde::{Deserialize, Serialize};

use crate::core::async_image::{AsyncImage, ImageLoader, RawImage};

#[derive(Serialize, Deserialize, Clone)]
pub struct Track {
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub duration: f32,
    #[serde(skip)]
    pub image: AsyncImage,
}

impl Track {
    pub fn default() -> Self {
        Track {
            file_path: String::new(),
            title: "Title".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            genre: "Genre".to_string(),
            duration: 0.,
            image: AsyncImage::default(),
        }
    }

    pub fn new(path: &str) -> Option<Self> {
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

        let (title, artist, album, genre, image_loader): (
            String,
            String,
            String,
            String,
            Box<dyn ImageLoader>,
        ) = if let Some(tag) = tag {
            let title = tag
                .title()
                .unwrap_or(Cow::Borrowed(fallback_title))
                .to_string();
            let artist = tag.artist().unwrap_or(Cow::Borrowed("Unknown")).to_string();
            let album = tag.album().unwrap_or(Cow::Borrowed("Unknown")).to_string();
            let genre = tag.genre().unwrap_or(Cow::Borrowed("Unknown")).to_string();

            let image_loader: Box<dyn ImageLoader> = tag
                .pictures()
                .iter()
                .find(|pic| pic.pic_type() == PictureType::CoverFront)
                .or_else(|| tag.pictures().first())
                .map(|pic| {
                    Box::new(RawImage {
                        data: pic.data().to_vec(),
                    }) as Box<dyn ImageLoader>
                })
                .unwrap_or_else(|| {
                    Box::new(RawImage {
                        data: include_bytes!("../../assets/default.png").to_vec(),
                    })
                });

            (title, artist, album, genre, image_loader)
        } else {
            (
                fallback_title.to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                Box::new(RawImage {
                    data: include_bytes!("../../assets/default.png").to_vec(),
                }),
            )
        };

        Some(Track {
            file_path: path.to_string(),
            title,
            artist,
            album,
            genre,
            duration,
            image: AsyncImage::new(image_loader),
        })
    }
}
