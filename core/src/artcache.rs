use dashmap::DashMap;
use blake3::{Hash, hash};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use std::{path::{Path, PathBuf}, sync::Arc};

use crate::Track;

const THUMBNAIL_SIZE: u32 = 512;

enum CacheState {
    /// Image is fully loaded and cached
    Ready(Arc<DynamicImage>),
    /// Image is currently being loaded
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

    /// Try to get an image from cache. Returns None if not cached or still loading.
    /// If None is returned and the image hasn't been requested yet, it will start
    /// loading it asynchronously in a background thread.
    pub fn get(&self, track: &Track) -> Option<Arc<DynamicImage>> {
        let art_id = track.metadata.art_id?;

        // Check if already in cache
        if let Some(entry) = self.cache.get(&art_id) {
            match entry.value() {
                CacheState::Ready(img) => return Some(img.clone()),
                CacheState::Loading => return None,
            }
        }

        // Not in cache - mark as loading and spawn background task
        if self.cache.insert(art_id, CacheState::Loading).is_none() {
            // We just inserted Loading state, so we're the ones to start loading
            let cache = self.cache.clone();
            let path = track.path.clone();

            // Spawn on rayon thread pool
            rayon::spawn(move || {
                Self::load_image_sync(cache, art_id, path);
            });
        }

        None
    }

    /// Load an image on a background thread and update the cache when done
    fn load_image_sync(cache: Arc<DashMap<Hash, CacheState>>, art_id: Hash, path: PathBuf) {
        match Self::extract_and_decode(&path) {
            Some((data, image)) => {
                // Verify the hash matches
                let actual_hash = hash(&data);
                if actual_hash != art_id {
                    // Hash mismatch - remove loading state
                    cache.remove(&art_id);
                    return;
                }

                let thumbnail = Self::resize_to_thumbnail(image);
                let arc_thumbnail = Arc::new(thumbnail);
                cache.insert(art_id, CacheState::Ready(arc_thumbnail));
            }
            None => {
                // Failed to load - remove loading state so it can be retried
                cache.remove(&art_id);
            }
        }
    }

    /// Get image directly by hash (if already cached)
    pub fn get_by_hash(&self, id: &Hash) -> Option<Arc<DynamicImage>> {
        self.cache.get(id).and_then(|entry| {
            match entry.value() {
                CacheState::Ready(img) => Some(img.clone()),
                CacheState::Loading => None,
            }
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
