use dashmap::DashMap;
use blake3::Hash;
use image::DynamicImage;
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use std::{path::Path, sync::Arc};

use crate::Track;

pub struct ArtCache {
    cache: DashMap<Hash, Arc<DynamicImage>>,
}

impl ArtCache {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    pub fn get_or_load(&self, track: &Track) -> Option<Arc<DynamicImage>> {
        if let Some(id) = track.metadata.art_id {
            if let Some(img) = self.cache.get(&id) {
                return Some(img.value().clone());
            }
        }

        // Need to extract from file
        let (image_data, image) = Self::extract_and_decode(&track.path)?;

        // Hash the image data to get unique ID
        let id = blake3::hash(&image_data);

        // Cache it
        let arc_image = Arc::new(image);
        self.cache.insert(id, arc_image.clone());

        // Note: We can't mutate track.metadata.artwork_id here since track is borrowed
        // The ID will be recomputed next time, but it will find it in cache

        Some(arc_image)
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

    pub fn clear(&self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }
}
