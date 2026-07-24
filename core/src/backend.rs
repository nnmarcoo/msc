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

use crate::analyzer::{AnalyzerBuilder, VisData, VisReader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackendState {
    Idle,
    Playing,
    Paused,
    Finished,
}

pub(crate) struct Backend {
    manager: AudioManager,
    sound: Option<StreamingSoundHandle<FromFileError>>,
    volume: f32,
    vis: Arc<VisReader>,
}

impl Backend {
    pub(crate) fn new() -> Result<Self, cpal::Error> {
        let (analyzer, vis) = AnalyzerBuilder::new();

        let settings = AudioManagerSettings {
            main_track_builder: MainTrackBuilder::new().with_effect(analyzer),
            ..AudioManagerSettings::default()
        };

        Ok(Backend {
            manager: AudioManager::<DefaultBackend>::new(settings)?,
            sound: None,
            volume: 1.0,
            vis,
        })
    }

    pub(crate) fn load_and_play(&mut self, path: &Path) -> Result<(), PlaybackError> {
        self.stop();

        let sound = StreamingSoundData::from_file(path)
            .map_err(PlaybackError::Load)?
            .volume(self.volume_db());

        self.sound = Some(self.manager.play(sound).map_err(PlaybackError::Play)?);
        Ok(())
    }

    pub(crate) fn resume(&mut self) {
        if let Some(sound) = &mut self.sound
            && matches!(
                sound.state(),
                PlaybackState::Paused | PlaybackState::Pausing
            )
        {
            sound.resume(Tween::default());
        }
    }

    pub(crate) fn pause(&mut self) {
        if let Some(sound) = &mut self.sound
            && sound.state() == PlaybackState::Playing
        {
            sound.pause(Tween {
                start_time: StartTime::Immediate,
                duration: Duration::from_millis(500),
                easing: Easing::OutPowi(2),
            });
        }
    }

    pub(crate) fn stop(&mut self) {
        if let Some(sound) = &mut self.sound {
            sound.stop(Tween::default());
        }
        self.sound = None;
    }

    pub(crate) fn seek(&mut self, position: f64) {
        if let Some(sound) = &mut self.sound {
            sound.seek_to(position);
        }
    }

    pub(crate) fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        let db = self.volume_db();
        if let Some(sound) = &mut self.sound {
            sound.set_volume(db, Tween::default());
        }
    }

    pub(crate) fn volume(&self) -> f32 {
        self.volume
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
            Some(sound) => match sound.state() {
                PlaybackState::Playing => BackendState::Playing,
                PlaybackState::Paused | PlaybackState::Pausing => BackendState::Paused,
                PlaybackState::Stopped => BackendState::Finished,
                _ => BackendState::Idle,
            },
        }
    }

    pub(crate) fn position(&self) -> f64 {
        self.sound
            .as_ref()
            .map_or(0.0, kira::sound::streaming::StreamingSoundHandle::position)
    }

    pub(crate) fn vis_data(&self) -> VisData {
        self.vis.read()
    }
}

#[derive(Debug, Error)]
pub enum PlaybackError {
    #[error("Failed to load audio file: {0}")]
    Load(FromFileError),
    #[error("Failed to play audio: {0}")]
    Play(PlaySoundError<FromFileError>),
}
