use crate::image_processing::{Colors, extract_colors};
use iced::widget::image::Handle;
use msc_core::extract_artwork_bytes;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

pub struct ArtEntry {
    pub handle: Handle,
    pub colors: Colors,
}

type CacheKey = (i64, u32, u32);

struct WorkItem {
    track_id: i64,
    path: PathBuf,
    width: u32,
    height: u32,
}

struct ArtResult {
    track_id: i64,
    width: u32,
    height: u32,
    handle: Handle,
    colors: Colors,
}

fn worker_loop(rx: Receiver<WorkItem>, tx: Sender<ArtResult>) {
    while let Ok(item) = rx.recv() {
        let Some(bytes) = extract_artwork_bytes(&item.path) else {
            continue;
        };
        let Ok(img) = image::load_from_memory(&bytes) else {
            continue;
        };

        let colors = extract_colors(&img);
        let img = img.resize(item.width, item.height, image::imageops::FilterType::Lanczos3);

        let rgba = img.into_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        let handle = Handle::from_rgba(w, h, rgba.into_raw());

        let _ = tx.send(ArtResult {
            track_id: item.track_id,
            width: item.width,
            height: item.height,
            handle,
            colors,
        });
    }
}

pub struct ArtCache {
    ready: HashMap<CacheKey, ArtEntry>,
    pending: HashSet<CacheKey>,
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
            pending: HashSet::new(),
            work_tx,
            result_rx,
            _worker: handle,
        }
    }

    pub fn poll(&mut self) {
        while let Ok(result) = self.result_rx.try_recv() {
            let key = (result.track_id, result.width, result.height);
            self.pending.remove(&key);
            self.ready.insert(
                key,
                ArtEntry {
                    handle: result.handle,
                    colors: result.colors,
                },
            );
        }
    }

    pub fn get_or_queue(
        &mut self,
        track_id: i64,
        path: &Path,
        width: u32,
        height: u32,
    ) -> Option<&ArtEntry> {
        if width == 0 || height == 0 {
            return None;
        }
        let key = (track_id, width, height);
        if self.ready.contains_key(&key) {
            return self.ready.get(&key);
        }
        if self.pending.insert(key) {
            let _ = self.work_tx.send(WorkItem {
                track_id,
                path: path.to_path_buf(),
                width,
                height,
            });
        }
        None
    }

    pub fn get(&self, track_id: i64, width: u32, height: u32) -> Option<&ArtEntry> {
        self.ready.get(&(track_id, width, height))
    }

    pub fn get_any(&self, track_id: i64) -> Option<&ArtEntry> {
        self.ready
            .iter()
            .find(|((tid, _, _), _)| *tid == track_id)
            .map(|(_, entry)| entry)
    }

    pub fn invalidate(&mut self) {
        self.ready.clear();
        self.pending.clear();
    }
}
