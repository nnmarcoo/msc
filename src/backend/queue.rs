use std::io::Cursor;

use kira::{
    manager::{AudioManager, AudioManagerSettings, DefaultBackend},
    sound::{
        streaming::{StreamingSoundData, StreamingSoundHandle},
        FromFileError, PlaybackState,
    },
};

use crate::constants::{DEFALT_AUDIO_BYTES, TWEEN_DEFAULT, TWEEN_INSTANT};

use super::{playlist::Playlist, track::Track};

pub struct Queue {
    tracks: Vec<Track>,
    current_index: Option<usize>,
    manager: AudioManager,
    sound: StreamingSoundHandle<FromFileError>,
}

impl Queue {
    pub fn new() -> Self {
        let mut manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();

        let cursor = Cursor::new(DEFALT_AUDIO_BYTES);

        let stream = StreamingSoundData::from_cursor(cursor).unwrap();
        let sound = manager.play(stream).unwrap();

        Queue {
            tracks: Vec::new(),
            current_index: None,
            sound,
            manager,
        }
    }

    pub fn from_playlist(playlist: Playlist) -> Self {
        let mut manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();

        let mut sound = if let Some(first_track) = playlist.tracks.get(0) {
            let stream = StreamingSoundData::from_file(&first_track.file_path).unwrap();
            manager.play(stream).unwrap()
        } else {
            let cursor = Cursor::new(DEFALT_AUDIO_BYTES);
            let stream = StreamingSoundData::from_cursor(cursor).unwrap();
            manager.play(stream).unwrap()
        };

        sound.pause(TWEEN_INSTANT);

        Queue {
            tracks: playlist.tracks,
            current_index: Some(0),
            manager,
            sound,
        }
    }

    pub fn toggle_playback(&mut self) {
        match self.sound.state() {
            PlaybackState::Playing => {
                self.sound.pause(TWEEN_DEFAULT);
            }
            PlaybackState::Paused => {
                self.sound.resume(TWEEN_DEFAULT);
            }
            _ => {}
        }
    }

    pub fn queue_playlist(&mut self, playlist: Playlist) {
        self.tracks.extend(playlist.tracks);
    }

    pub fn queue_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn queue_track_next(&mut self, track: Track) {
        if let Some(index) = self.current_index {
            self.tracks.insert(index + 1, track);
        } else {
            self.tracks.insert(0, track);
        }
    }

    pub fn current_track(&mut self) -> Option<&mut Track> {
        self.current_index
            .and_then(|index| self.tracks.get_mut(index))
    }

    pub fn play_next_track(&mut self, volume: f64) {
        self.play_adjacent_track(true, volume);
    }

    pub fn play_previous_track(&mut self, volume: f64) {
        self.play_adjacent_track(false, volume);
    }

    fn play_adjacent_track(&mut self, forward: bool, volume: f64) {
        if self.tracks.is_empty() {
            return;
        }
        self.current_index = self.current_index.map(|i| {
            if forward {
                (i + 1) % self.tracks.len()
            } else {
                (i + self.tracks.len() - 1) % self.tracks.len()
            }
        });
        self.start(volume);
    }

    pub fn start(&mut self, volume: f64) {
        self.sound.stop(TWEEN_DEFAULT);
        if let Some(index) = self.current_index {
            let stream =
                StreamingSoundData::from_file(&self.tracks.get(index).unwrap().file_path).unwrap();
            self.sound = self.manager.play(stream).unwrap();
            self.sound.set_volume(volume, TWEEN_DEFAULT);
        }
    }

    pub fn seek(&mut self, pos: f32) {
        self.sound.seek_to(pos as f64);
    }

    pub fn position(&self) -> f32 {
        self.sound.position() as f32
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.sound.set_volume(volume as f64, TWEEN_DEFAULT);
    }

    pub fn is_playing(&self) -> bool {
        self.sound.state() == PlaybackState::Playing
    }
}
