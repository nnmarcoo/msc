use dashmap::DashMap;
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
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
    original: Arc<DynamicImage>,
    is_loading: Arc<AtomicBool>,
    pending_size: Arc<AtomicU32>,
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
        original: Arc<DynamicImage>,
        size: u32,
        is_loading: Arc<AtomicBool>,
        pending_size: Arc<AtomicU32>,
    },
}

pub struct ArtCache {
    cache: Arc<DashMap<String, CacheState>>,
    work_queue: Arc<Mutex<Sender<WorkItem>>>,
    _workers: Vec<JoinHandle<()>>,
}

impl ArtCache {
    pub fn new() -> Self {
        const MAX_THREADS: usize = 4;

        let (tx, rx) = mpsc::channel::<WorkItem>();
        let rx = Arc::new(Mutex::new(rx));
        let cache = Arc::new(DashMap::new());

        let mut workers = Vec::new();
        for _ in 0..MAX_THREADS {
            let rx = Arc::clone(&rx);
            let cache = Arc::clone(&cache);

            let handle = thread::spawn(move || {
                Self::worker_loop(rx, cache);
            });
            workers.push(handle);
        }

        Self {
            cache,
            work_queue: Arc::new(Mutex::new(tx)),
            _workers: workers,
        }
    }

    fn worker_loop(rx: Arc<Mutex<Receiver<WorkItem>>>, cache: Arc<DashMap<String, CacheState>>) {
        loop {
            let work_item = {
                let receiver = match rx.lock() {
                    Ok(r) => r,
                    Err(_) => break,
                };

                match receiver.recv() {
                    Ok(item) => item,
                    Err(_) => break,
                }
            };

            match work_item {
                WorkItem::LoadInitial {
                    cache_key,
                    path,
                    size,
                } => {
                    Self::load_image_sync(cache.clone(), cache_key, path, size);
                }
                WorkItem::GenerateSize {
                    cache_key,
                    original,
                    size,
                    is_loading,
                    pending_size,
                } => {
                    Self::generate_size_sync(
                        cache.clone(),
                        cache_key,
                        original,
                        size,
                        is_loading,
                        pending_size,
                    );
                }
            }
        }
    }

    pub fn get(&self, track: &Track, size: u32) -> Option<(RgbaImage, Colors)> {
        let cache_key = Self::cache_key(track)?;

        if let Some(entry) = self.cache.get(&cache_key) {
            match entry.value() {
                CacheState::Ready(cached) => {
                    // Check if we have this exact size
                    if let Some(image) = cached.images.get(&size) {
                        return Some((image.clone(), cached.colors));
                    }

                    // Update the pending size
                    cached.pending_size.store(size, Ordering::Relaxed);

                    // Find the closest available size
                    let closest = cached
                        .images
                        .range(size..)
                        .next()
                        .or_else(|| cached.images.iter().next_back())
                        .map(|(_, img)| img.clone());

                    if let Some(image) = closest {
                        // Queue generation of the requested size if not already loading
                        if !cached.is_loading.swap(true, Ordering::Relaxed) {
                            let work_item = WorkItem::GenerateSize {
                                cache_key: cache_key.clone(),
                                original: cached.original.clone(),
                                size,
                                is_loading: cached.is_loading.clone(),
                                pending_size: cached.pending_size.clone(),
                            };

                            if let Ok(queue) = self.work_queue.lock() {
                                let _ = queue.send(work_item);
                            }
                        }

                        return Some((image, cached.colors));
                    }
                }
                CacheState::Loading => return None,
            }
        }

        // Not in cache - queue it for loading
        if self
            .cache
            .insert(cache_key.clone(), CacheState::Loading)
            .is_none()
        {
            let work_item = WorkItem::LoadInitial {
                cache_key,
                path: track.path().clone(),
                size,
            };

            if let Ok(queue) = self.work_queue.lock() {
                let _ = queue.send(work_item);
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

    fn generate_size_sync(
        cache: Arc<DashMap<String, CacheState>>,
        cache_key: String,
        original: Arc<DynamicImage>,
        size: u32,
        is_loading: Arc<AtomicBool>,
        pending_size: Arc<AtomicU32>,
    ) {
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
            is_loading.store(false, Ordering::Relaxed);
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
