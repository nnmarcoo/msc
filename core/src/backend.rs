use std::path::Path;

use kira::{
    backend::cpal,
    sound::{
        streaming::{StreamingSoundData, StreamingSoundHandle},
        FromFileError, PlaybackState,
    },
    AudioManager, AudioManagerSettings, DefaultBackend, PlaySoundError, Tween,
};

#[derive(Debug)]
pub enum PlaybackError {
    LoadError(FromFileError),
    PlayError(PlaySoundError<FromFileError>),
}

impl std::fmt::Display for PlaybackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaybackError::LoadError(e) => write!(f, "Failed to load audio file: {}", e),
            PlaybackError::PlayError(e) => write!(f, "Failed to play audio: {}", e),
        }
    }
}

impl std::error::Error for PlaybackError {}

pub struct Backend {
    manager: AudioManager,
    sound: Option<StreamingSoundHandle<FromFileError>>,
    volume: f32,
}

impl Backend {
    pub fn new() -> Result<Self, cpal::Error> {
        Ok(Backend {
            manager: AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?,
            sound: None,
            volume: 0.5,
        })
    }

    // should I set the start position?
    pub fn load_and_play(&mut self, path: &Path) -> Result<(), PlaybackError> {
        self.stop();

        let sound_data = StreamingSoundData::from_file(path)
            .map_err(PlaybackError::LoadError)?
            .volume(self.volume);

        let handle = self
            .manager
            .play(sound_data)
            .map_err(PlaybackError::PlayError)?;

        self.sound = Some(handle);
        Ok(())
    }

    pub fn play(&mut self) {
        if let Some(sound) = &mut self.sound {
            if sound.state() == PlaybackState::Paused {
                sound.resume(Tween::default());
            }
        }
    }

    pub fn pause(&mut self) {
        if let Some(sound) = &mut self.sound {
            if sound.state() == PlaybackState::Playing {
                sound.pause(Tween::default());
            }
        }
    }

    pub fn toggle_playback(&mut self) {
        if let Some(sound) = &mut self.sound {
            match sound.state() {
                PlaybackState::Playing => sound.pause(Tween::default()),
                PlaybackState::Paused => sound.resume(Tween::default()),
                _ => {}
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(sound) = &mut self.sound {
            sound.stop(Tween::default());
            self.sound = None;
        }
    }

    pub fn seek(&mut self, pos: f64) {
        if let Some(sound) = &mut self.sound {
            sound.seek_to(pos);
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = 45. * volume.log10();
        if let Some(sound) = &mut self.sound {
            sound.set_volume(self.volume, Tween::default());
        }
    }

    pub fn is_playing(&self) -> bool {
        if let Some(sound) = &self.sound {
            sound.state() == PlaybackState::Playing
        } else {
            false
        }
    }

    pub fn is_stopped(&self) -> bool {
        if let Some(sound) = &self.sound {
            sound.state() == PlaybackState::Stopped
        } else {
            true
        }
    }

    pub fn position(&self) -> f64 {
        if let Some(sound) = &self.sound {
            sound.position()
        } else {
            0.
        }
    }
}
