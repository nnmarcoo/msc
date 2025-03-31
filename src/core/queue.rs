use kira::{
    sound::{streaming::StreamingSoundHandle, FromFileError, PlaybackState},
    AudioManager, AudioManagerSettings, DefaultBackend, Tween,
};

use super::playlist::Playlist;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Queue {
    tracks: Vec<String>,
    current_index: Option<usize>,
    #[serde(skip)]
    manager: Option<AudioManager>,
    #[serde(skip)]
    sound: Option<StreamingSoundHandle<FromFileError>>,
}

impl Queue {
    pub fn new() -> Self {
        Queue {
            tracks: Vec::new(),
            current_index: None,
            manager: Some(
                AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
            ),
            sound: None,
        }
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
    }

    pub fn track(&mut self, track: String) {
        self.tracks.push(track);
    }

    pub fn playlist(&mut self, playlist: Playlist) {
        self.tracks.splice(0..0, playlist.tracks);
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
