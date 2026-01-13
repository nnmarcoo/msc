use blake3::Hash as Blake3Hash;
use dashmap::DashMap;
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use lofty::{file::TaggedFileExt, picture::PictureType, probe::Probe};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ImageHash([u8; 32]);

impl From<Blake3Hash> for ImageHash {
    fn from(hash: Blake3Hash) -> Self {
        ImageHash(*hash.as_bytes())
    }
}

impl ImageHash {
    fn compute(data: &[u8]) -> Self {
        blake3::hash(data).into()
    }
}

#[derive(Clone, Debug)]
struct TrackPictureMap {
    pictures: BTreeMap<u8, ImageHash>,
}

impl TrackPictureMap {
    fn new() -> Self {
        Self {
            pictures: BTreeMap::new(),
        }
    }

    fn insert(&mut self, pic_type: PictureType, hash: ImageHash) {
        self.pictures.insert(Self::type_to_id(&pic_type), hash);
    }

    fn get(&self, pic_type: PictureType) -> Option<ImageHash> {
        self.pictures.get(&Self::type_to_id(&pic_type)).copied()
    }

    fn get_any(&self) -> Option<ImageHash> {
        self.pictures.values().next().copied()
    }

    fn type_to_id(pic_type: &PictureType) -> u8 {
        match pic_type {
            PictureType::Other => 0,
            PictureType::Icon => 1,
            PictureType::OtherIcon => 2,
            PictureType::CoverFront => 3,
            PictureType::CoverBack => 4,
            PictureType::Leaflet => 5,
            PictureType::Media => 6,
            PictureType::LeadArtist => 7,
            PictureType::Artist => 8,
            PictureType::Conductor => 9,
            PictureType::Band => 10,
            PictureType::Composer => 11,
            PictureType::Lyricist => 12,
            PictureType::RecordingLocation => 13,
            PictureType::DuringRecording => 14,
            PictureType::DuringPerformance => 15,
            PictureType::ScreenCapture => 16,
            PictureType::BrightFish => 17,
            PictureType::Illustration => 18,
            PictureType::BandLogo => 19,
            PictureType::PublisherLogo => 20,
            PictureType::Undefined(val) => *val,
            _ => 255,
        }
    }
}

#[derive(Clone)]
struct CachedImage {
    original: Arc<DynamicImage>,
    sizes: BTreeMap<u32, RgbaImage>,
    colors: Colors,
    is_loading: Arc<AtomicBool>,
}

enum ImageState {
    Ready(CachedImage),
    Loading,
}

#[derive(Debug)]
enum WorkItem {
    LoadTrackPictures {
        track_key: String,
        path: PathBuf,
        initial_size: u32,
    },
    GenerateSize {
        image_hash: ImageHash,
        size: u32,
        is_loading: Arc<AtomicBool>,
    },
}

fn extract_pictures_from_file(path: &Path) -> Option<Vec<(PictureType, Vec<u8>)>> {
    let file = Probe::open(path).ok()?.read().ok()?;
    let tag = file.primary_tag().or_else(|| file.first_tag())?;

    let pictures: Vec<_> = tag
        .pictures()
        .iter()
        .map(|pic| (pic.pic_type(), pic.data().to_vec()))
        .collect();

    (!pictures.is_empty()).then_some(pictures)
}

pub struct ArtCache {
    track_pictures: Arc<DashMap<String, TrackPictureMap>>,
    images: Arc<DashMap<ImageHash, ImageState>>,
    work_queue: Sender<WorkItem>,
    _worker: JoinHandle<()>,
}

impl ArtCache {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let track_pictures = Arc::new(DashMap::new());
        let images = Arc::new(DashMap::new());

        let worker_track_pictures = Arc::clone(&track_pictures);
        let worker_images = Arc::clone(&images);

        let handle = thread::spawn(move || {
            Self::worker_loop(rx, worker_track_pictures, worker_images);
        });

        Self {
            track_pictures,
            images,
            work_queue: tx,
            _worker: handle,
        }
    }

    fn worker_loop(
        rx: Receiver<WorkItem>,
        track_pictures: Arc<DashMap<String, TrackPictureMap>>,
        images: Arc<DashMap<ImageHash, ImageState>>,
    ) {
        while let Ok(work_item) = rx.recv() {
            match work_item {
                WorkItem::LoadTrackPictures {
                    track_key,
                    path,
                    initial_size,
                } => {
                    Self::load_track_pictures_sync(
                        &track_pictures,
                        &images,
                        track_key,
                        path,
                        initial_size,
                    );
                }
                WorkItem::GenerateSize {
                    image_hash,
                    size,
                    is_loading,
                } => {
                    Self::generate_size_sync(&images, image_hash, size, is_loading);
                }
            }
        }
    }

    pub fn get(
        &self,
        track: &Track,
        size: u32,
        pic_type: Option<PictureType>,
    ) -> Option<(RgbaImage, Colors)> {
        let track_key = Self::track_key(track);

        if let Some(pic_map_entry) = self.track_pictures.get(&track_key) {
            let image_hash = if let Some(requested_type) = pic_type {
                pic_map_entry
                    .get(requested_type)
                    .or_else(|| pic_map_entry.get_any())
            } else {
                pic_map_entry
                    .get(PictureType::CoverFront)
                    .or_else(|| pic_map_entry.get_any())
            }?;

            return self.get_image_by_hash(image_hash, size);
        }

        if self
            .track_pictures
            .insert(track_key.clone(), TrackPictureMap::new())
            .is_none()
        {
            let _ = self.work_queue.send(WorkItem::LoadTrackPictures {
                track_key,
                path: track.path().clone(),
                initial_size: size,
            });
        }

        None
    }

    fn get_image_by_hash(&self, image_hash: ImageHash, size: u32) -> Option<(RgbaImage, Colors)> {
        let entry = self.images.get(&image_hash)?;

        match entry.value() {
            ImageState::Ready(cached) => {
                if let Some(image) = cached.sizes.get(&size) {
                    return Some((image.clone(), cached.colors));
                }

                let closest = cached
                    .sizes
                    .range(size..)
                    .next()
                    .or_else(|| cached.sizes.iter().next_back())
                    .map(|(_, img)| img.clone())?;

                if !cached.is_loading.swap(true, Ordering::AcqRel) {
                    let _ = self.work_queue.send(WorkItem::GenerateSize {
                        image_hash,
                        size,
                        is_loading: cached.is_loading.clone(),
                    });
                }

                Some((closest, cached.colors))
            }
            ImageState::Loading => None,
        }
    }

    #[inline]
    fn track_key(track: &Track) -> String {
        track.path().to_string_lossy().into_owned()
    }

    fn load_track_pictures_sync(
        track_pictures: &DashMap<String, TrackPictureMap>,
        images: &DashMap<ImageHash, ImageState>,
        track_key: String,
        path: PathBuf,
        initial_size: u32,
    ) {
        let Some(pictures) = extract_pictures_from_file(&path) else {
            track_pictures.remove(&track_key);
            return;
        };

        let mut pic_map = TrackPictureMap::new();

        for (pic_type, raw_data) in pictures {
            let image_hash = ImageHash::compute(&raw_data);
            pic_map.insert(pic_type, image_hash);

            if images.contains_key(&image_hash) {
                continue;
            }

            images.insert(image_hash, ImageState::Loading);

            if let Ok(image) = image::load_from_memory(&raw_data) {
                let colors = extract_colors(&image);
                let rgba_image = Self::create_rgba_image(&image, initial_size);

                let mut sizes = BTreeMap::new();
                sizes.insert(initial_size, rgba_image);

                let cached = CachedImage {
                    original: Arc::new(image),
                    sizes,
                    colors,
                    is_loading: Arc::new(AtomicBool::new(false)),
                };

                images.insert(image_hash, ImageState::Ready(cached));
            } else {
                images.remove(&image_hash);
            }
        }

        track_pictures.insert(track_key, pic_map);
    }

    fn generate_size_sync(
        images: &DashMap<ImageHash, ImageState>,
        image_hash: ImageHash,
        size: u32,
        is_loading: Arc<AtomicBool>,
    ) {
        let original = images.get(&image_hash).and_then(|entry| {
            if let ImageState::Ready(cached) = entry.value() {
                if cached.sizes.contains_key(&size) {
                    is_loading.store(false, Ordering::Release);
                    return None;
                }
                Some(Arc::clone(&cached.original))
            } else {
                None
            }
        });

        if let Some(original) = original {
            let rgba_image = Self::create_rgba_image(&original, size);
            if let Some(mut entry) = images.get_mut(&image_hash) {
                if let ImageState::Ready(cached) = entry.value_mut() {
                    cached.sizes.insert(size, rgba_image);
                }
            }
        }

        is_loading.store(false, Ordering::Release);
    }

    fn create_rgba_image(image: &DynamicImage, max_size: u32) -> RgbaImage {
        let rgba = image
            .resize(max_size, max_size, FilterType::Lanczos3)
            .to_rgba8();
        let (width, height) = rgba.dimensions();

        RgbaImage {
            width,
            height,
            data: Arc::new(rgba.into_raw()),
        }
    }
}

impl Default for ArtCache {
    fn default() -> Self {
        Self::new()
    }
}
