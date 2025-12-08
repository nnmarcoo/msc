use dashmap::DashMap;
use blake3::{Hash, hash};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use std::{path::Path, sync::Arc};

use crate::Track;

// redo this it sucks

const THUMBNAIL_SIZE: u32 = 512;

pub struct ArtCache {
    cache: DashMap<Hash, Arc<DynamicImage>>,
}

impl ArtCache {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    // non blocking
    pub fn try_get(&self, track: &Track) -> Option<Arc<DynamicImage>> {
        track.metadata.art_id
            .and_then(|id| self.cache.get(&id))
            .map(|entry| entry.value().clone())
    }

    // blocking
    pub fn get_or_load(&self, track: &Track) -> Option<Arc<DynamicImage>> {
        if let Some(id) = track.metadata.art_id {
            if let Some(img) = self.cache.get(&id) {
                return Some(img.value().clone());
            }
        }

        let (image_data, image) = Self::extract_and_decode(&track.path)?;

        let id = hash(&image_data);

        if let Some(img) = self.cache.get(&id) {
            return Some(img.value().clone());
        }

        let thumbnail = Self::resize_to_thumbnail(image);

        let arc_thumbnail = Arc::new(thumbnail);
        self.cache.insert(id, arc_thumbnail.clone());

        Some(arc_thumbnail)
    }

    pub fn get(&self, id: &Hash) -> Option<Arc<DynamicImage>> {
        self.cache.get(id).map(|entry| entry.value().clone())
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
