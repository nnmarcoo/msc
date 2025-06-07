use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use blake3::Hash;
use dashmap::DashMap;
use egui::ResizeDirection;

use super::{helps::collect_audio_files, playlist::Playlist, queue::Queue, track::Track};

#[derive(serde::Deserialize, serde::Serialize, PartialEq)]
pub enum View {
    Covers,
    Settings,
    Library,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(skip)]
    pub is_dragging: bool,
    #[serde(skip)]
    pub is_maximized: bool,
    #[serde(skip)]
    pub resizing: Option<ResizeDirection>,
    #[serde(skip)]
    pub library: Arc<DashMap<Hash, Track>>,
    #[serde(skip)]
    pub library_loaded: Arc<AtomicBool>,
    #[serde(skip)]
    pub is_initialized: bool,
    pub audio_directory: String,
    pub view: View,
    pub playlists: Vec<Playlist>,
    pub queue: Queue,
}

impl Default for State {
    fn default() -> Self {
        State {
            is_dragging: false,
            is_maximized: false,
            is_initialized: false,
            resizing: None,
            audio_directory: Default::default(),
            library: Default::default(),
            view: View::Covers,
            playlists: Vec::new(),
            queue: Queue::new(),
            library_loaded: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl State {
    pub fn init(&mut self) {
        if self.is_initialized {
            return;
        }
        self.is_initialized = true;

        self.queue.init_audio_manager();
        self.start_loading_library();
    }

    pub fn start_loading_library(&self) {
        self.library_loaded.store(false, Ordering::SeqCst);

        let library = Arc::clone(&self.library);
        let done_flag = Arc::clone(&self.library_loaded);
        let dir = self.audio_directory.clone();

        thread::spawn(move || {
            let collected = collect_audio_files(Path::new(&dir));
            library.clear();
            for (hash, track) in collected.into_iter() {
                library.insert(hash, track);
            }
            done_flag.store(true, Ordering::SeqCst);
        });
    }
}
