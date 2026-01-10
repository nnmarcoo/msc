use dashmap::DashMap;
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
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
    path: PathBuf,
    is_loading: Arc<AtomicBool>,
}

enum CacheState {
    Ready(CachedImages),
    Loading,
}

#[derive(Debug)]
enum WorkItem {
    LoadInitial {
        cache_key: String,
        path: PathBuf,
        size: u32,
    },
    GenerateSize {
        cache_key: String,
        path: PathBuf,
        size: u32,
        is_loading: Arc<AtomicBool>,
    },
}

pub struct ArtCache {
    cache: Arc<DashMap<String, CacheState>>,
    work_queue: Sender<WorkItem>,
    _worker: JoinHandle<()>,
}

impl ArtCache {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<WorkItem>();
        let cache = Arc::new(DashMap::new());

        let worker_cache = Arc::clone(&cache);
        let handle = thread::spawn(move || {
            Self::worker_loop(rx, worker_cache);
        });

        Self {
            cache,
            work_queue: tx,
            _worker: handle,
        }
    }

    fn worker_loop(rx: Receiver<WorkItem>, cache: Arc<DashMap<String, CacheState>>) {
        while let Ok(work_item) = rx.recv() {
            match work_item {
                WorkItem::LoadInitial {
                    cache_key,
                    path,
                    size,
                } => {
                    Self::load_image_sync(&cache, cache_key, path, size);
                }
                WorkItem::GenerateSize {
                    cache_key,
                    path,
                    size,
                    is_loading,
                } => {
                    Self::generate_size_sync(&cache, cache_key, path, size, is_loading);
                }
            }
        }
    }

    pub fn get(&self, track: &Track, size: u32) -> Option<(RgbaImage, Colors)> {
        let cache_key = Self::cache_key(track)?;

        if let Some(entry) = self.cache.get(&cache_key) {
            match entry.value() {
                CacheState::Ready(cached) => {
                    if let Some(image) = cached.images.get(&size) {
                        return Some((image.clone(), cached.colors));
                    }

                    let closest = cached
                        .images
                        .range(size..)
                        .next()
                        .or_else(|| cached.images.iter().next_back())
                        .map(|(_, img)| img.clone());

                    if let Some(image) = closest {
                        if !cached.is_loading.swap(true, Ordering::AcqRel) {
                            if self
                                .work_queue
                                .send(WorkItem::GenerateSize {
                                    cache_key,
                                    path: cached.path.clone(),
                                    size,
                                    is_loading: cached.is_loading.clone(),
                                })
                                .is_err()
                            {
                                eprintln!("ArtCache: worker thread disconnected");
                            }
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
            if self
                .work_queue
                .send(WorkItem::LoadInitial {
                    cache_key,
                    path: track.path().clone(),
                    size,
                })
                .is_err()
            {
                eprintln!("ArtCache: worker thread disconnected");
            }
        }

        None
    }

    fn cache_key(track: &Track) -> Option<String> {
        let album = track.album()?;
        let artist = track.album_artist().unwrap_or("Unknown");
        Some(format!("{}|{}", album, artist))
    }

    fn load_image_sync(
        cache: &DashMap<String, CacheState>,
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
                    path,
                    is_loading: Arc::new(AtomicBool::new(false)),
                };

                cache.insert(cache_key, CacheState::Ready(cached));
            }
            None => {
                cache.remove(&cache_key);
            }
        }
    }

    fn generate_size_sync(
        cache: &DashMap<String, CacheState>,
        cache_key: String,
        path: PathBuf,
        size: u32,
        is_loading: Arc<AtomicBool>,
    ) {
        if let Some(image) = Self::extract_and_decode(&path) {
            let rgba_image = Self::create_rgba_image(&image, size);

            if let Some(mut entry) = cache.get_mut(&cache_key) {
                if let CacheState::Ready(cached) = entry.value_mut() {
                    cached.images.insert(size, rgba_image);
                }
            }
        }

        is_loading.store(false, Ordering::Release);
    }

    fn create_rgba_image(image: &DynamicImage, max_size: u32) -> RgbaImage {
        let (img_width, img_height) = image.dimensions();

        let rgba = if img_width <= max_size && img_height <= max_size {
            image.to_rgba8()
        } else {
            image
                .resize(max_size, max_size, FilterType::Lanczos3)
                .to_rgba8()
        };

        let (width, height) = rgba.dimensions();

        RgbaImage {
            width,
            height,
            data: Arc::new(rgba.into_raw()),
        }
    }

    fn extract_and_decode(path: &Path) -> Option<DynamicImage> {
        let file = Probe::open(path).ok()?.read().ok()?;
        let tag = file.primary_tag().or_else(|| file.first_tag())?;
        let picture = tag.pictures().first()?;
        image::load_from_memory(picture.data()).ok()
    }
}
