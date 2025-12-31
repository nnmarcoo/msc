use dashmap::DashMap;
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    thread,
};

use super::image_processing::{Colors, extract_colors};
use crate::Track;

#[derive(Clone)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub data: Arc<Vec<u8>>,
}

#[derive(Clone)]
struct CachedImages {
    images: BTreeMap<u32, RgbaImage>,
    colors: Colors,
    original: Arc<DynamicImage>,
    is_loading: Arc<AtomicBool>,
    pending_size: Arc<AtomicU32>,
}

enum CacheState {
    Ready(CachedImages),
    Loading,
}

pub struct ArtCache {
    cache: Arc<DashMap<String, CacheState>>,
    loading_count: Arc<AtomicU32>,
}

impl ArtCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            loading_count: Arc::new(AtomicU32::new(0)),
        }
    }

    const MAX_THREADS: u32 = 4;

    pub fn get(&self, track: &Track, size: u32) -> Option<(RgbaImage, Colors)> {
        let cache_key = Self::cache_key(track)?;

        if let Some(entry) = self.cache.get(&cache_key) {
            match entry.value() {
                CacheState::Ready(cached) => {
                    if let Some(image) = cached.images.get(&size) {
                        return Some((image.clone(), cached.colors));
                    }

                    cached.pending_size.store(size, Ordering::Relaxed);

                    let closest = cached
                        .images
                        .range(size..)
                        .next()
                        .or_else(|| cached.images.iter().next_back())
                        .map(|(_, img)| img.clone());

                    if let Some(image) = closest {
                        if !cached.is_loading.swap(true, Ordering::Relaxed) {
                            let cache = self.cache.clone();
                            let cache_key = cache_key.clone();
                            let original = cached.original.clone();
                            let is_loading = cached.is_loading.clone();
                            let pending_size = cached.pending_size.clone();
                            std::thread::spawn(move || {
                                Self::generate_pending_size(
                                    cache,
                                    cache_key,
                                    original,
                                    is_loading,
                                    pending_size,
                                );
                            });
                        }

                        return Some((image, cached.colors));
                    }
                }
                CacheState::Loading => return None,
            }
        }

        if self
            .cache
            .insert(cache_key.clone(), CacheState::Loading)
            .is_none()
        {
            if self.loading_count.load(Ordering::Relaxed) < Self::MAX_THREADS {
                self.loading_count.fetch_add(1, Ordering::Relaxed);

                let cache = self.cache.clone();
                let path = track.path().clone();
                let loading_count = self.loading_count.clone();

                std::thread::spawn(move || {
                    Self::load_image_sync(cache, cache_key, path, size);
                    loading_count.fetch_sub(1, Ordering::Relaxed);
                });
            }
        }

        None
    }

    fn cache_key(track: &Track) -> Option<String> {
        let album = track.album()?;
        let artist = track.album_artist().or_else(|| track.track_artist())?;
        Some(format!("{}|{}", album, artist))
    }

    fn load_image_sync(
        cache: Arc<DashMap<String, CacheState>>,
        cache_key: String,
        path: PathBuf,
        initial_size: u32,
    ) {
        match Self::extract_and_decode(&path) {
            Some(image) => {
                let colors = extract_colors(&image);
                let rgba_image = Self::create_rgba_image(&image, initial_size);

                let mut images = BTreeMap::new();
                images.insert(initial_size, rgba_image);

                let cached = CachedImages {
                    images,
                    colors,
                    original: Arc::new(image),
                    is_loading: Arc::new(AtomicBool::new(false)),
                    pending_size: Arc::new(AtomicU32::new(initial_size)),
                };

                cache.insert(cache_key, CacheState::Ready(cached));
            }
            None => {
                cache.remove(&cache_key);
            }
        }
    }

    fn generate_pending_size(
        cache: Arc<DashMap<String, CacheState>>,
        cache_key: String,
        original: Arc<DynamicImage>,
        is_loading: Arc<AtomicBool>,
        pending_size: Arc<AtomicU32>,
    ) {
        let size = pending_size.load(Ordering::Relaxed);
        let rgba_image = Self::create_rgba_image(&original, size);

        let should_continue = if let Some(mut entry) = cache.get_mut(&cache_key) {
            if let CacheState::Ready(cached) = entry.value_mut() {
                cached.images.insert(size, rgba_image);

                let current_pending = pending_size.load(Ordering::Relaxed);
                current_pending != size && !cached.images.contains_key(&current_pending)
            } else {
                false
            }
        } else {
            false
        };

        if should_continue {
            thread::spawn(move || {
                Self::generate_pending_size(cache, cache_key, original, is_loading, pending_size);
            });
        } else {
            is_loading.store(false, Ordering::Relaxed);
        }
    }

    fn create_rgba_image(image: &DynamicImage, max_size: u32) -> RgbaImage {
        let resized = Self::resize_image(image, max_size);
        let rgba = resized.to_rgba8();
        let (width, height) = rgba.dimensions();
        let bytes = rgba.into_raw();

        RgbaImage {
            width,
            height,
            data: Arc::new(bytes),
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

    fn resize_image(image: &DynamicImage, max_size: u32) -> DynamicImage {
        let (width, height) = image.dimensions();

        if width <= max_size && height <= max_size {
            return image.clone();
        }

        image.resize(max_size, max_size, FilterType::Lanczos3)
    }
}
