use kira::{
    sound::{
        streaming::{StreamingSoundData, StreamingSoundHandle},
        FromFileError, PlaybackState,
    },
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween,
};

use super::{playlist::Playlist, track::Track};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Queue {
    tracks: Vec<Track>,
    current_index: usize,
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
            manager: Some(
                AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
            ),
            sound: None,
        }
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
    }

    pub fn shuffle(&mut self) {}

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

    pub fn set_volume(&mut self, volume: f32) {
        if let Some(sound) = &mut self.sound {
            sound.set_volume(Decibels::from(volume), Tween::default());
        }
    }

    pub fn play_next(&mut self, volume: f32) {
        self.current_index = (self.current_index + 1) % self.tracks.len();
        self.start(volume);
    }

    pub fn play_previous(&mut self, volume: f32) {
        self.current_index = (self.current_index + self.tracks.len() - 1) % self.tracks.len();
        self.start(volume);
    }

    pub fn start(&mut self, volume: f32) {
        if let Some(sound) = &mut self.sound {
            sound.stop(Tween::default());
            let stream = StreamingSoundData::from_file(
                &self.tracks.get(self.current_index).unwrap().file_path,
            )
            .unwrap()
            .volume(Decibels::from(volume));
            self.sound = Some(self.manager.as_mut().unwrap().play(stream).unwrap());
        }
    }

    pub fn track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn playlist(&mut self, playlist: Playlist) {
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
        }
    }
}
