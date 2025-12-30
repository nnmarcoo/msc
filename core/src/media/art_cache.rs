use dashmap::DashMap;
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use super::image_processing::{Colors, extract_colors};
use crate::Track;

const THUMBNAIL_SIZE: u32 = 1024;

#[derive(Clone)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub data: Arc<Vec<u8>>,
}

enum CacheState {
    Ready { image: RgbaImage, colors: Colors },
    Loading,
}

pub struct ArtCache {
    cache: Arc<DashMap<String, CacheState>>,
}

impl ArtCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }

    pub fn get(&self, track: &Track) -> Option<(RgbaImage, Colors)> {
        let cache_key = Self::cache_key(track)?;

        if let Some(entry) = self.cache.get(&cache_key) {
            match entry.value() {
                CacheState::Ready { image, colors } => return Some((image.clone(), *colors)),
                CacheState::Loading => return None,
            }
        }

        if self
            .cache
            .insert(cache_key.clone(), CacheState::Loading)
            .is_none()
        {
            let cache = self.cache.clone();
            let path = track.path().clone();

            rayon::spawn(move || {
                Self::load_image_sync(cache, cache_key, path);
            });
        }

        None
    }

    fn cache_key(track: &Track) -> Option<String> {
        let album = track.album()?;
        let artist = track.album_artist().or_else(|| track.track_artist())?;
        Some(format!("{}|{}", album, artist))
    }

    fn load_image_sync(cache: Arc<DashMap<String, CacheState>>, cache_key: String, path: PathBuf) {
        match Self::extract_and_decode(&path) {
            Some(image) => {
                let colors = extract_colors(&image);
                let thumbnail = Self::resize_to_thumbnail(image);
                let rgba = thumbnail.to_rgba8();
                let (width, height) = rgba.dimensions();
                let bytes = rgba.into_raw();

                let rgba_image = RgbaImage {
                    width,
                    height,
                    data: Arc::new(bytes),
                };

                cache.insert(
                    cache_key,
                    CacheState::Ready {
                        image: rgba_image,
                        colors,
                    },
                );
            }
            None => {
                cache.remove(&cache_key);
            }
        }
    }

    fn extract_and_decode(path: &Path) -> Option<DynamicImage> {
        let file = Probe::open(path).ok()?.read().ok()?;
        let tag = file.primary_tag().or_else(|| file.first_tag())?;
        let picture = tag.pictures().first()?;
        let data = picture.data();
        let image = image::load_from_memory(data).ok()?;
        Some(image)
    }

    fn resize_to_thumbnail(image: DynamicImage) -> DynamicImage {
        let (width, height) = image.dimensions();

        if width <= THUMBNAIL_SIZE && height <= THUMBNAIL_SIZE {
            return image;
        }

        image.resize(THUMBNAIL_SIZE, THUMBNAIL_SIZE, FilterType::Lanczos3)
    }
}
