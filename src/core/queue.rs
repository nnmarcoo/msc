use blake3::Hash;
use kira::{
    sound::{
        streaming::{StreamingSoundData, StreamingSoundHandle},
        FromFileError, PlaybackState,
    },
    AudioManager, AudioManagerSettings, DefaultBackend, Tween,
};

use crate::{core::helps::amp_to_db, state::State};

use super::{playlist::Playlist, track::Track};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Queue {
    pub tracks: Vec<Track>, // should be Hash not Track?
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
        Queue {
            tracks: Vec::new(),
            current_index: 0,
            volume: 0.5,
            timeline_pos: 0.,
            manager: None,
            sound: None,
        }
    }

    pub fn init_audio_manager(&mut self) {
        if self.manager.is_none() {}
        self.manager =
            Some(AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap())
    }

    pub fn get_current_track(&self) -> Track {
        if let Some(track) = self.tracks.get(self.current_index) {
            return track.clone();
        }
        Track::default()
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current_index = 0;
        if let Some(sound) = &mut self.sound {
            sound.stop(Tween::default());
        }
    }

    pub fn shuffle(&mut self) {
        todo!()
    }

    pub fn seek(&mut self, pos: f32) {
        if let Some(sound) = &mut self.sound {
            sound.seek_to(pos as f64);
        }
    }

    pub fn position(&self) -> f32 {
        if let Some(sound) = &self.sound {
            return sound.position() as f32;
        }
        0.
    }

    pub fn is_playing(&self) -> bool {
        if let Some(sound) = &self.sound {
            return sound.state() == PlaybackState::Playing;
        }
        false
    }

    pub fn update_volume(&mut self) {
        if let Some(sound) = &mut self.sound {
            sound.set_volume(amp_to_db(self.volume), Tween::default());
        }
    }

    pub fn play_next(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        self.current_index = (self.current_index + 1) % self.tracks.len();
        self.timeline_pos = 0.;
        self.start();
    }

    pub fn play_previous(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        self.current_index = (self.current_index + self.tracks.len() - 1) % self.tracks.len();
        self.timeline_pos = 0.;
        self.start();
    }

    pub fn start(&mut self) { // bad
        if let Some(sound) = &mut self.sound {
            sound.stop(Tween::default());
        }

        if self.tracks.is_empty() || self.current_index >= self.tracks.len() {
            self.sound = None;
            return;
        }

        let path = &self.tracks[self.current_index].file_path;

        let stream_result = StreamingSoundData::from_file(path).map(|data| {
            data.start_position(self.timeline_pos as f64)
                .volume(amp_to_db(self.volume))
        });

        match stream_result {
            Ok(stream) => {
                if let Some(manager) = &mut self.manager {
                    match manager.play(stream) {
                        Ok(sound) => {
                            self.sound = Some(sound);
                            return;
                        }
                        Err(e) => {
                            eprintln!("Failed to play track '{}': {}", path, e);
                            self.tracks.remove(self.current_index);
                            if self.current_index > 0 {
                                self.current_index -= 1;
                            }
                            self.start();
                        }
                    }
                } else {
                    eprintln!("No audio manager available.");
                }
            }
            Err(e) => {
                eprintln!("Failed to load track '{}': {}", path, e);
                self.tracks.remove(self.current_index);
                if self.current_index > 0 {
                    self.current_index -= 1;
                }
                self.start();
            }
        }
    }

    pub fn queue_track(&mut self, hash: &Hash, state: &State) {
        if let Some(track) = state.library.get(hash) {
            self.tracks.push(track.value().clone());
        }
    }

    pub fn play(&mut self, track: Track) {
        if self.tracks.is_empty() {
            self.tracks.push(track);
        } else {
            self.tracks.insert(self.current_index + 1, track);
        }
        self.play_next();
    }

    pub fn queue_playlist(&mut self, playlist: Playlist) {
        todo!("Playlists shold use the Track type not String")
        //self.tracks.splice(0..0, playlist.tracks);
    }

    pub fn toggle_playback(&mut self) {
        if let Some(sound) = &mut self.sound {
            match sound.state() {
                PlaybackState::Playing => {
                    sound.pause(Tween::default());
                }
                PlaybackState::Paused => {
                    sound.resume(Tween::default());
                }
                _ => {}
            }
        } else {
            self.start();
        }
    }
}
