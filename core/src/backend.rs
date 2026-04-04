use crossbeam::atomic::AtomicCell;
use std::{path::Path, sync::Arc, time::Duration};
use thiserror::Error;

use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, Easing, PlaySoundError, StartTime, Tween,
    backend::cpal,
    sound::{
        FromFileError, PlaybackState,
        streaming::{StreamingSoundData, StreamingSoundHandle},
    },
    track::MainTrackBuilder,
};

use crate::audio_analyzer::{AudioAnalyzerBuilder, VisData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackendState {
    Idle,
    Playing,
    Paused,
    Finished,
}

pub struct Backend {
    manager: AudioManager,
    sound: Option<StreamingSoundHandle<FromFileError>>,
    volume: f32,
    visualization_data: Arc<AtomicCell<VisData>>,
}

impl Backend {
    pub fn new() -> Result<Self, cpal::Error> {
        let (analyzer_builder, visualization_data) = AudioAnalyzerBuilder::new();

        let settings = AudioManagerSettings {
            main_track_builder: MainTrackBuilder::new().with_effect(analyzer_builder),
            ..AudioManagerSettings::default()
        };

        Ok(Backend {
            manager: AudioManager::<DefaultBackend>::new(settings)?,
            sound: None,
            volume: 1.0,
            visualization_data,
        })
    }

    pub fn load_and_play(&mut self, path: &Path) -> Result<(), PlaybackError> {
        self.stop();

        let sound_data = StreamingSoundData::from_file(path)
            .map_err(PlaybackError::LoadError)?
            .volume(self.volume_db());

        let handle = self
            .manager
            .play(sound_data)
            .map_err(PlaybackError::PlayError)?;

        self.sound = Some(handle);
        Ok(())
    }

    pub fn play(&mut self) {
        if let Some(sound) = &mut self.sound {
            let state = sound.state();
            if state == PlaybackState::Paused || state == PlaybackState::Pausing {
                sound.resume(Tween::default());
            }
        }
    }

    pub fn pause(&mut self) {
        if let Some(sound) = &mut self.sound {
            if sound.state() == PlaybackState::Playing {
                sound.pause(Tween {
                    start_time: StartTime::Immediate,
                    duration: Duration::from_millis(500),
                    easing: Easing::OutPowi(2),
                });
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
        self.volume = volume.clamp(0.0, 1.0);
        let db = self.volume_db();
        if let Some(sound) = &mut self.sound {
            sound.set_volume(db, Tween::default());
        }
    }

    fn volume_db(&self) -> f32 {
        if self.volume <= 0.0 {
            -60.0
        } else {
            28.0 * self.volume.log10()
        }
    }

    pub(crate) fn state(&self) -> BackendState {
        match &self.sound {
            None => BackendState::Idle,
            Some(s) => match s.state() {
                PlaybackState::Playing => BackendState::Playing,
                PlaybackState::Paused | PlaybackState::Pausing => BackendState::Paused,
                PlaybackState::Stopped => BackendState::Finished,
                _ => BackendState::Idle,
            },
        }
    }

    pub fn is_playing(&self) -> bool {
        self.state() == BackendState::Playing
    }

    pub fn position(&self) -> f64 {
        if let Some(sound) = &self.sound {
            sound.position()
        } else {
            0.0
        }
    }

    pub fn vis_data(&self) -> VisData {
        self.visualization_data.load()
    }
}

#[derive(Debug, Error)]
pub enum PlaybackError {
    #[error("Failed to load audio file: {0}")]
    LoadError(FromFileError),
    #[error("Failed to play audio: {0}")]
    PlayError(PlaySoundError<FromFileError>),
}
