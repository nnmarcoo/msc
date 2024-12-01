use std::io::Cursor;

use kira::{
    manager::{AudioManager, AudioManagerSettings, DefaultBackend},
    sound::{
        streaming::{StreamingSoundData, StreamingSoundHandle},
        FromFileError, PlaybackState,
    },
    tween::Tween,
};

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

        let bytes = include_bytes!("../../assets/setup/placeholder.mp3");
        let cursor = Cursor::new(bytes);

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
            let bytes = include_bytes!("../../assets/setup/placeholder.mp3");
            let cursor = Cursor::new(bytes);
            let stream = StreamingSoundData::from_cursor(cursor).unwrap();
            manager.play(stream).unwrap()
        };

        sound.pause(Tween::default());

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
                self.sound.pause(Default::default());
            }
            PlaybackState::Paused | PlaybackState::Stopped => {
                self.sound.resume(Default::default());
            }
            _ => {}
        }
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.sound.set_volume(volume, Tween::default());
    }

    pub fn is_playing(&self) -> bool {
        self.sound.state() == PlaybackState::Playing
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

    pub fn current_track(&self) -> Option<&Track> {
        self.current_index.and_then(|index| self.tracks.get(index))
    }

    pub fn play_next_track(&mut self, volume: f64) {
        self.increment_index();
        self.start(volume);
    }

    pub fn play_previous_track(&mut self, volume: f64) {
        self.decrement_index();
        self.start(volume);
    }

    pub fn seek(&mut self, pos: f64) {
        self.sound.seek_to(pos);
    }

    pub fn position(&self) -> f64 {
        self.sound.position()
    }

    pub fn start(&mut self, volume: f64) {
        self.sound.stop(Tween::default());
        if let Some(i) = self.current_index {
            let stream =
                StreamingSoundData::from_file(&self.tracks.get(i).unwrap().file_path).unwrap();
            self.sound = self.manager.play(stream).unwrap();
            self.sound.set_volume(volume, Tween::default());
        }
    }

    fn increment_index(&mut self) {
        if let Some(i) = self.current_index {
            if i < self.tracks.len() {
                self.current_index = Some(i + 1);
            } else {
                self.current_index = Some(0);
            }
        }
    }

    fn decrement_index(&mut self) {
        if let Some(i) = self.current_index {
            if i > 0 {
                self.current_index = Some(i - 1);
            } else {
                self.current_index = Some(self.tracks.len() - 1);
            }
        }
    }
}
