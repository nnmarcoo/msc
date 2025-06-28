use blake3::Hash;
use dashmap::{
    mapref::one::{Ref, RefMut},
    DashMap,
};
use kira::{
    sound::{
        streaming::{StreamingSoundData, StreamingSoundHandle},
        FromFileError, PlaybackState,
    },
    AudioManager, AudioManagerSettings, DefaultBackend, Tween,
};

use super::track::Track;
use crate::core::helps::amp_to_db;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Queue {
    pub tracks: Vec<Hash>,
    pub volume: f32,
    pub timeline_pos: f32,
    pub current_index: usize,

    #[serde(skip)]
    manager: Option<AudioManager>,

    #[serde(skip)]
    sound: Option<StreamingSoundHandle<FromFileError>>,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            current_index: 0,
            volume: 0.5,
            timeline_pos: 0.,
            manager: None,
            sound: None,
        }
    }

    pub fn init_audio_manager(&mut self) {
        if self.manager.is_none() {
            self.manager =
                Some(AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap());
        }
    }

    pub fn get_track_ref<'a>(
        &'a self,
        map: &'a DashMap<Hash, Track>,
    ) -> Option<Ref<'a, Hash, Track>> {
        self.tracks
            .get(self.current_index)
            .and_then(|hash| map.get(hash))
    }

    pub fn get_track_mut_ref<'a>(
        &'a self,
        map: &'a DashMap<Hash, Track>,
    ) -> Option<RefMut<'a, Hash, Track>> {
        self.tracks
            .get(self.current_index)
            .and_then(|hash| map.get_mut(hash))
    }

    pub fn get_track_copy(&self, map: &DashMap<Hash, Track>) -> Track {
        self.tracks
            .get(self.current_index)
            .and_then(|hash| map.get(hash))
            .map(|track_ref| track_ref.clone())
            .unwrap_or_else(Track::default)
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current_index = 0;
        self.timeline_pos = 0.;
        if let Some(sound) = &mut self.sound {
            sound.stop(Tween::default());
        }
    }

    pub fn shuffle(&mut self) {
        todo!("Implement shuffle using rand::seq::SliceRandom");
    }

    pub fn seek(&mut self, pos: f32) {
        if let Some(sound) = &mut self.sound {
            sound.seek_to(pos as f64);
        }
    }

    pub fn position(&self) -> f32 {
        self.sound
            .as_ref()
            .map(|s| s.position() as f32)
            .unwrap_or(0.0)
    }

    pub fn is_playing(&self) -> bool {
        self.sound
            .as_ref()
            .map(|s| s.state() == PlaybackState::Playing)
            .unwrap_or(false)
    }

    pub fn update_volume(&mut self) {
        if let Some(sound) = &mut self.sound {
            sound.set_volume(amp_to_db(self.volume), Tween::default());
        }
    }

    pub fn play_next(&mut self, library: &DashMap<Hash, Track>) {
        if self.tracks.is_empty() {
            return;
        }
        self.current_index = (self.current_index + 1) % self.tracks.len();
        self.timeline_pos = 0.;
        self.start(library);
    }

    pub fn play_previous(&mut self, library: &DashMap<Hash, Track>) {
        if self.tracks.is_empty() {
            return;
        }
        self.current_index = (self.current_index + self.tracks.len() - 1) % self.tracks.len();
        self.timeline_pos = 0.;
        self.start(library);
    }

    pub fn start(&mut self, library: &DashMap<Hash, Track>) {
        if let Some(sound) = &mut self.sound {
            sound.stop(Tween::default());
        }

        let path = self
            .get_track_ref(library)
            .map(|track| track.file_path.clone())
            .unwrap_or_default();

        if path.is_empty() {
            return;
        }

        let stream_result = StreamingSoundData::from_file(&path).map(|data| {
            data.start_position(self.timeline_pos as f64)
                .volume(amp_to_db(self.volume))
        });

        match (stream_result, self.manager.as_mut()) {
            (Ok(stream), Some(manager)) => match manager.play(stream) {
                Ok(sound) => self.sound = Some(sound),
                Err(e) => {
                    eprintln!("Failed to play '{}': {}", path, e);
                    self.remove_invalid_track();
                    self.start(library);
                }
            },
            (Err(e), _) => {
                eprintln!("Failed to load '{}': {}", path, e);
                self.remove_invalid_track();
                self.start(library);
            }
            (_, None) => {
                eprintln!("No audio manager available.");
            }
        }
    }

    fn remove_invalid_track(&mut self) {
        if self.current_index < self.tracks.len() {
            self.tracks.remove(self.current_index);
            if self.current_index > 0 {
                self.current_index -= 1;
            }
        }
    }

    pub fn queue_track(&mut self, hash: Hash, library: &DashMap<Hash, Track>) {
        if library.contains_key(&hash) {
            self.tracks.push(hash);
        }
    }

    pub fn play(&mut self, hash: Hash, library: &DashMap<Hash, Track>) {
        if self.tracks.is_empty() {
            self.tracks.push(hash);
            self.current_index = 0;
        } else {
            self.tracks.insert(self.current_index + 1, hash);
            self.current_index += 1;
        }
        self.timeline_pos = 0.;
        self.start(library);
    }

    pub fn toggle_playback(&mut self, library: &DashMap<Hash, Track>) {
        if let Some(sound) = &mut self.sound {
            match sound.state() {
                PlaybackState::Playing => sound.pause(Tween::default()),
                PlaybackState::Paused => sound.resume(Tween::default()),
                _ => {}
            }
        } else {
            self.start(library);
        }
    }
}
