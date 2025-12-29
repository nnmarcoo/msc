use blake3::{Hash, hash};
use dashmap::DashMap;
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::Track;
use crate::image_processing::{Colors, extract_colors};

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
    cache: Arc<DashMap<Hash, CacheState>>,
}

impl ArtCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }

    pub fn get(&self, track: &Track) -> Option<(RgbaImage, Colors)> {
        let art_id = *track.art_id()?;

        if let Some(entry) = self.cache.get(&art_id) {
            match entry.value() {
                CacheState::Ready { image, colors } => return Some((image.clone(), *colors)),
                CacheState::Loading => return None,
            }
        }

        if self.cache.insert(art_id, CacheState::Loading).is_none() {
            let cache = self.cache.clone();
            let path = track.path().clone();

            rayon::spawn(move || {
                Self::load_image_sync(cache, art_id, path);
            });
        }

        None
    }

    fn load_image_sync(cache: Arc<DashMap<Hash, CacheState>>, art_id: Hash, path: PathBuf) {
        match Self::extract_and_decode(&path) {
            Some((data, image)) => {
                let actual_hash = hash(&data);
                if actual_hash != art_id {
                    cache.remove(&art_id);
                    return;
                }

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
                    art_id,
                    CacheState::Ready {
                        image: rgba_image,
                        colors,
                    },
                );
            }
            None => {
                cache.remove(&art_id);
            }
        }
    }

    pub fn get_by_hash(&self, id: &Hash) -> Option<(RgbaImage, Colors)> {
        self.cache.get(id).and_then(|entry| match entry.value() {
            CacheState::Ready { image, colors } => Some((image.clone(), *colors)),
            CacheState::Loading => None,
        })
    }

    fn extract_and_decode(path: &Path) -> Option<(Vec<u8>, DynamicImage)> {
        let file = Probe::open(path).ok()?.read().ok()?;
        let tag = file.primary_tag().or_else(|| file.first_tag())?;
        let picture = tag.pictures().first()?;
        let data = picture.data().to_vec();
        let image = image::load_from_memory(&data).ok()?;
        Some((data, image))
    }

    fn resize_to_thumbnail(image: DynamicImage) -> DynamicImage {
        let (width, height) = image.dimensions();

        if width <= THUMBNAIL_SIZE && height <= THUMBNAIL_SIZE {
            return image;
        }

        image.resize(THUMBNAIL_SIZE, THUMBNAIL_SIZE, FilterType::Lanczos3)
    }

    pub fn clear(&self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }
}
