use blake3::Hash as Blake3Hash;
use dashmap::DashMap;
use image::{DynamicImage, imageops::FilterType};
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

const MAX_PICTURES: usize = 21;

use super::image_processing::{Colors, extract_colors};
use crate::Track;

#[derive(Clone)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub data: Arc<[u8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ImageHash([u8; 32]);

impl ImageHash {
    fn compute(data: &[u8]) -> Self {
        blake3::hash(data).into()
    }
}

impl From<Blake3Hash> for ImageHash {
    fn from(hash: Blake3Hash) -> Self {
        Self(*hash.as_bytes())
    }
}

enum ImageState {
    Ready(CachedImage),
    Loading,
}

#[derive(Clone)]
struct CachedImage {
    original: Arc<DynamicImage>,
    sizes: BTreeMap<u32, RgbaImage>,
    colors: Colors,
    is_loading: Arc<AtomicBool>,
}

#[derive(Clone, Debug)]
struct TrackPictureMap {
    data: [(u8, ImageHash); MAX_PICTURES],
    len: u8,
}

impl TrackPictureMap {
    fn new() -> Self {
        Self {
            data: [(0, ImageHash([0; 32])); MAX_PICTURES],
            len: 0,
        }
    }

    fn insert(&mut self, pic_type: PictureType, hash: ImageHash) {
        if (self.len as usize) < MAX_PICTURES {
            self.data[self.len as usize] = (picture_type_id(pic_type), hash);
            self.len += 1;
        }
    }

    fn get(&self, pic_type: PictureType) -> Option<ImageHash> {
        let id = picture_type_id(pic_type);
        self.data[..self.len as usize]
            .iter()
            .find(|(k, _)| *k == id)
            .map(|(_, h)| *h)
    }

    fn get_any(&self) -> Option<ImageHash> {
        (self.len > 0).then(|| self.data[0].1)
    }
}

fn picture_type_id(pic_type: PictureType) -> u8 {
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
        PictureType::Undefined(val) => val,
        _ => 255,
    }
}

#[derive(Debug)]
enum WorkItem {
    Load {
        track_key: String,
        path: PathBuf,
        initial_size: u32,
    },
    Resize {
        hash: ImageHash,
        size: u32,
        is_loading: Arc<AtomicBool>,
    },
}

fn worker_loop(
    rx: Receiver<WorkItem>,
    track_pictures: Arc<DashMap<String, TrackPictureMap>>,
    images: Arc<DashMap<ImageHash, ImageState>>,
) {
    while let Ok(item) = rx.recv() {
        match item {
            WorkItem::Load {
                track_key,
                path,
                initial_size,
            } => load_track_pictures(&track_pictures, &images, track_key, path, initial_size),
            WorkItem::Resize {
                hash,
                size,
                is_loading,
            } => generate_size(&images, hash, size, is_loading),
        }
    }
}

fn load_track_pictures(
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
        let hash = ImageHash::compute(&raw_data);
        pic_map.insert(pic_type, hash);

        if images.contains_key(&hash) {
            continue;
        }

        images.insert(hash, ImageState::Loading);

        if let Ok(image) = image::load_from_memory(&raw_data) {
            let colors = extract_colors(&image);
            let rgba = create_rgba(&image, initial_size);

            let mut sizes = BTreeMap::new();
            sizes.insert(initial_size, rgba);

            images.insert(
                hash,
                ImageState::Ready(CachedImage {
                    original: Arc::new(image),
                    sizes,
                    colors,
                    is_loading: Arc::new(AtomicBool::new(false)),
                }),
            );
        } else {
            images.remove(&hash);
        }
    }

    track_pictures.insert(track_key, pic_map);
}

fn generate_size(
    images: &DashMap<ImageHash, ImageState>,
    hash: ImageHash,
    size: u32,
    is_loading: Arc<AtomicBool>,
) {
    let original = images.get(&hash).and_then(|entry| {
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
        let rgba = create_rgba(&original, size);
        if let Some(mut entry) = images.get_mut(&hash) {
            if let ImageState::Ready(cached) = entry.value_mut() {
                cached.sizes.insert(size, rgba);
            }
        }
    }

    is_loading.store(false, Ordering::Release);
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

fn create_rgba(image: &DynamicImage, max_size: u32) -> RgbaImage {
    let rgba = image
        .resize(max_size, max_size, FilterType::Lanczos3)
        .to_rgba8();
    let (width, height) = rgba.dimensions();

    RgbaImage {
        width,
        height,
        data: rgba.into_raw().into(),
    }
}

pub struct ArtCache {
    track_pictures: Arc<DashMap<String, TrackPictureMap>>,
    images: Arc<DashMap<ImageHash, ImageState>>,
    work_tx: Sender<WorkItem>,
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
            worker_loop(rx, worker_track_pictures, worker_images);
        });

        Self {
            track_pictures,
            images,
            work_tx: tx,
            _worker: handle,
        }
    }

    pub fn get(
        &self,
        track: &Track,
        size: u32,
        pic_type: Option<PictureType>,
    ) -> Option<(RgbaImage, Colors)> {
        let track_key = track.path().to_string_lossy().into_owned();

        if let Some(pic_map) = self.track_pictures.get(&track_key) {
            let hash = match pic_type {
                Some(t) => pic_map.get(t).or_else(|| pic_map.get_any()),
                None => pic_map
                    .get(PictureType::CoverFront)
                    .or_else(|| pic_map.get_any()),
            }?;

            return self.get_by_hash(hash, size);
        }

        if self
            .track_pictures
            .insert(track_key.clone(), TrackPictureMap::new())
            .is_none()
        {
            let _ = self.work_tx.send(WorkItem::Load {
                track_key,
                path: track.path().clone(),
                initial_size: size,
            });
        }

        None
    }

    fn get_by_hash(&self, hash: ImageHash, size: u32) -> Option<(RgbaImage, Colors)> {
        let entry = self.images.get(&hash)?;

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
                    let _ = self.work_tx.send(WorkItem::Resize {
                        hash,
                        size,
                        is_loading: cached.is_loading.clone(),
                    });
                }

                Some((closest, cached.colors))
            }
            ImageState::Loading => None,
        }
    }
}

impl Default for ArtCache {
    fn default() -> Self {
        Self::new()
    }
}
