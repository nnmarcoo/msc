use blake3::Hash;
use kira::{
    sound::{
        streaming::{StreamingSoundData, StreamingSoundHandle},
        FromFileError, PlaybackState,
    },
    AudioManager, AudioManagerSettings, DefaultBackend, Tween,
};

use crate::structs::State;

use super::{playlist::Playlist, track::Track};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Queue {
    pub tracks: Vec<Track>, // should be Hash not Track
    current_index: usize,
    pub volume: f32,
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
            manager: None,
            sound: None,
        }
    }

    pub fn init_audio_manager(&mut self) {
        if self.manager.is_none() {}
        self.manager =
            Some(AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap())
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
    }

    pub fn shuffle(&mut self) {
        todo!()
    }

    pub fn seek(&mut self, pos: f32) {
        if let Some(sound) = &mut self.sound {
            sound.seek_to(pos as f64);
        }
    }

    pub fn is_playing(&self) -> bool {
        if let Some(sound) = &self.sound {
            return sound.state() == PlaybackState::Playing;
        }
        false
    }

    pub fn update_volume(&mut self) {
        if let Some(sound) = &mut self.sound {
            sound.set_volume(self.volume, Tween::default());
        }
    }

    pub fn play_next(&mut self) {
        self.current_index = (self.current_index + 1) % self.tracks.len();
        self.start();
    }

    pub fn play_previous(&mut self) {
        self.current_index = (self.current_index + self.tracks.len() - 1) % self.tracks.len();
        self.start();
    }

    pub fn start(&mut self) {
        if let Some(sound) = &mut self.sound {
            sound.stop(Tween::default());
        }

        let stream =
            StreamingSoundData::from_file(&self.tracks.get(self.current_index).unwrap().file_path)
                .unwrap()
                .volume(self.volume);

        if let Some(manager) = &mut self.manager {
            self.sound = Some(manager.play(stream).unwrap());
        }
    }

    pub fn queue_track(&mut self, hash: &Hash, state: &State) {
        if let Some(track) = state.library.get(hash) {
            self.tracks.push(track.value().clone());
        }
    }

    pub fn play(&mut self, track: Track) {
        self.tracks.insert(self.current_index - 1, track);
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
