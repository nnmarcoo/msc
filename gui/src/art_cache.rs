use crate::image_processing::{Colors, extract_colors};
use iced::widget::image::Handle;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};
use verse_core::extract_artwork_bytes;

const EVICT_GRACE_FRAMES: u64 = 32;

pub struct ArtEntry {
    pub handle: Handle,
    pub colors: Colors,
}

type CacheKey = (i64, u32, u32);

struct WorkItem {
    key: CacheKey,
    path: PathBuf,
    generation: u64,
}

struct ArtResult {
    key: CacheKey,
    generation: u64,
    handle: Handle,
    colors: Colors,
}

fn worker_loop(rx: Receiver<WorkItem>, tx: Sender<ArtResult>) {
    while let Ok(item) = rx.recv() {
        let (_, width, height) = item.key;
        let Some(bytes) = extract_artwork_bytes(&item.path) else {
            continue;
        };
        let Ok(img) = image::load_from_memory(&bytes) else {
            continue;
        };

        let box_w = width.min(img.width());
        let box_h = height.min(img.height());
        let img = img.resize(box_w, box_h, image::imageops::FilterType::Lanczos3);
        let colors = extract_colors(&img);

        let rgba = img.into_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        let handle = Handle::from_rgba(w, h, rgba.into_raw());

        let _ = tx.send(ArtResult {
            key: item.key,
            generation: item.generation,
            handle,
            colors,
        });
    }
}

pub struct ArtCache {
    ready: HashMap<CacheKey, ArtEntry>,
    by_track: HashMap<i64, Vec<CacheKey>>,
    pending: HashSet<CacheKey>,
    last_wanted: HashMap<CacheKey, u64>,
    frame: u64,
    generation: u64,
    work_tx: Sender<WorkItem>,
    result_rx: Receiver<ArtResult>,
    _worker: JoinHandle<()>,
}

impl ArtCache {
    pub fn new() -> Self {
        let (work_tx, work_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let handle = thread::spawn(move || worker_loop(work_rx, result_tx));
        Self {
            ready: HashMap::new(),
            by_track: HashMap::new(),
            pending: HashSet::new(),
            last_wanted: HashMap::new(),
            frame: 0,
            generation: 0,
            work_tx,
            result_rx,
            _worker: handle,
        }
    }

    pub fn poll(&mut self) {
        while let Ok(result) = self.result_rx.try_recv() {
            self.pending.remove(&result.key);
            if result.generation != self.generation
                || !self.last_wanted.contains_key(&result.key)
            {
                continue;
            }
            if self
                .ready
                .insert(
                    result.key,
                    ArtEntry {
                        handle: result.handle,
                        colors: result.colors,
                    },
                )
                .is_none()
            {
                self.by_track.entry(result.key.0).or_default().push(result.key);
            }
        }
    }

    pub fn get_or_queue(&mut self, track_id: i64, path: &Path, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        let key = (track_id, width, height);
        self.last_wanted.insert(key, self.frame);
        if self.ready.contains_key(&key) {
            return;
        }
        if self.pending.insert(key) {
            let _ = self.work_tx.send(WorkItem {
                key,
                path: path.to_path_buf(),
                generation: self.generation,
            });
        }
    }

    pub fn get(&self, track_id: i64, width: u32, height: u32) -> Option<&ArtEntry> {
        self.ready.get(&(track_id, width, height))
    }

    pub fn get_any(&self, track_id: i64) -> Option<&ArtEntry> {
        self.by_track
            .get(&track_id)?
            .iter()
            .find_map(|key| self.ready.get(key))
    }

    pub fn evict(&mut self) {
        let frame = self.frame;
        let stale: Vec<CacheKey> = self
            .last_wanted
            .iter()
            .filter(|&(_, &last)| last + EVICT_GRACE_FRAMES <= frame)
            .map(|(&key, _)| key)
            .collect();

        for key in stale {
            self.last_wanted.remove(&key);
            self.ready.remove(&key);
            self.pending.remove(&key);
            if let Some(sizes) = self.by_track.get_mut(&key.0) {
                sizes.retain(|k| *k != key);
                if sizes.is_empty() {
                    self.by_track.remove(&key.0);
                }
            }
        }

        self.frame += 1;
    }

    pub fn invalidate(&mut self) {
        self.ready.clear();
        self.by_track.clear();
        self.pending.clear();
        self.last_wanted.clear();
        self.generation += 1;
    }
}
