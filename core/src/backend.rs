//! The kira audio backend.
//!
//! `seek` works while paused and must leave playback paused. kira consumes
//! `seek_to` on its decode thread, whose loop exits only on `Stopped` and so
//! keeps reading commands through a pause; the position moves without a frame
//! being played. An earlier version here resumed and re-paused around the seek
//! to force the command through, which was unnecessary and audibly restarted
//! playback on every scrub.
//!
//! A seek made while paused still replays roughly 0.37s of the old position on
//! resume. kira's decode thread seeks but never drains the frame ringbuffer it
//! has already filled, so audio decoded before the seek sits ahead of the new
//! frames. Playing, that is inaudible because the buffer drains continuously;
//! paused, nothing drains it. Re-issuing the seek on resume was tried and did
//! not help, since the stale frames are already queued and a second seek does not
//! remove them. The fix has to drain that buffer, which kira does not expose.
//!
//! `pause` fades over 500ms to soften a deliberate pause, which is why
//! `PlaybackState::Pausing` counts as paused wherever state is inspected: the
//! sound is still draining for half a second after the call returns.
//!
//! `state` maps every kira variant explicitly rather than falling back to a
//! catch-all. `Resuming` and `Stopping` are both audibly playing, but a `_ =>
//! Idle` arm reported them as idle, so a caller polling for "is it playing"
//! answered no during a fade. Listing every variant means a new one in kira
//! fails to compile here instead of quietly becoming idle.
//!
//! None of these states change the instant a call returns: `pause`, `resume`,
//! and `stop` write commands the audio thread applies later, so `state` lags a
//! moment behind. Anything that must react immediately to a transition has to
//! watch app-side state instead, not poll this.
//!
//! Volume is a linear 0..`VOLUME_MAX` scale converted to dB by `VOLUME_SLOPE *
//! log10`. `VOLUME_MAX` is unity, so the top of the scale is the file as
//! mastered and every level below it attenuates. The scale once ran to 2.0,
//! amplifying past unity; that clips, kira does not limit for us, and a volume
//! control that can distort what it plays is a trap rather than a feature.
//! `volume_db` is a free function so the curve is testable without an audio
//! device.

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

pub const VOLUME_MAX: f32 = 1.0;
const VOLUME_SLOPE: f32 = 28.0;
const SILENT_DB: f32 = -60.0;

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
        self.volume = volume.clamp(0.0, VOLUME_MAX);
        let db = self.volume_db();
        if let Some(sound) = &mut self.sound {
            sound.set_volume(db, Tween::default());
        }
    }

    pub(crate) fn volume(&self) -> f32 {
        self.volume
    }

    fn volume_db(&self) -> f32 {
        volume_db(self.volume)
    }

    pub(crate) fn state(&self) -> BackendState {
        match &self.sound {
            None => BackendState::Idle,
            Some(sound) => match sound.state() {
                PlaybackState::Playing | PlaybackState::Resuming | PlaybackState::Stopping => {
                    BackendState::Playing
                }
                PlaybackState::Paused | PlaybackState::Pausing | PlaybackState::WaitingToResume => {
                    BackendState::Paused
                }
                PlaybackState::Stopped => BackendState::Finished,
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

fn volume_db(volume: f32) -> f32 {
    if volume <= 0.0 {
        SILENT_DB
    } else {
        VOLUME_SLOPE * volume.log10()
    }
}

#[derive(Debug, Error)]
pub enum PlaybackError {
    #[error("Failed to load audio file: {0}")]
    Load(FromFileError),
    #[error("Failed to play audio: {0}")]
    Play(PlaySoundError<FromFileError>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_is_the_file_as_mastered() {
        assert!(
            volume_db(1.0).abs() < 0.001,
            "1.0 should be 0 dB, got {}",
            volume_db(1.0)
        );
    }

    #[test]
    fn the_top_of_the_scale_is_unity() {
        assert!(
            volume_db(VOLUME_MAX).abs() < 0.001,
            "the loudest level should be the file as mastered, got {}",
            volume_db(VOLUME_MAX)
        );
    }

    #[test]
    fn the_scale_never_amplifies() {
        for step in 0..=100 {
            let level = VOLUME_MAX * step as f32 / 100.0;
            assert!(
                volume_db(level) <= 0.001,
                "level {level} gave positive gain {}",
                volume_db(level)
            );
        }
    }

    #[test]
    fn quieter_than_unity_attenuates() {
        assert!(volume_db(0.5) < 0.0);
        assert!(volume_db(0.1) < volume_db(0.5));
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn silence_does_not_ask_for_negative_infinity() {
        assert_eq!(volume_db(0.0), SILENT_DB);
        assert!(volume_db(0.0).is_finite());
    }

    #[test]
    fn the_curve_rises_across_the_whole_scale() {
        let mut previous = volume_db(0.0);
        for step in 1u8..=20 {
            let level = f32::from(step) / 10.0;
            let db = volume_db(level);
            assert!(
                db > previous,
                "level {level} gave {db}, not above the previous {previous}"
            );
            previous = db;
        }
    }

    #[test]
    fn each_decade_of_the_scale_is_a_fixed_number_of_decibels() {
        let decade = volume_db(1.0) - volume_db(0.1);
        assert!(
            (decade - VOLUME_SLOPE).abs() < 0.001,
            "a tenfold change should be {VOLUME_SLOPE} dB, got {decade}"
        );
    }
}
