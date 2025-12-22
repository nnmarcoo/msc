use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use std::sync::mpsc::{Receiver, Sender, channel};

pub struct MediaSession {
    controls: MediaControls,
    event_rx: Receiver<MediaControlEvent>,
}

impl MediaSession {
    pub fn new(hwnd: Option<*mut std::ffi::c_void>) -> Result<Self, String> {
        let config = PlatformConfig {
            display_name: "MSC",
            dbus_name: "msc",
            hwnd,
        };

        let (event_tx, event_rx): (Sender<MediaControlEvent>, Receiver<MediaControlEvent>) =
            channel();

        let mut controls = MediaControls::new(config)
            .map_err(|e| format!("Failed to create media controls: {:?}", e))?;

        controls
            .attach(move |event| {
                let _ = event_tx.send(event);
            })
            .map_err(|e| format!("Failed to attach event handler: {:?}", e))?;

        Ok(MediaSession { controls, event_rx })
    }

    pub fn set_metadata(&mut self, title: &str, artist: &str, album: &str, duration: Option<f64>) {
        let metadata = MediaMetadata {
            title: Some(title),
            artist: Some(artist),
            album: Some(album),
            duration: duration.map(|d| std::time::Duration::from_secs_f64(d)),
            cover_url: None,
        };

        let _ = self.controls.set_metadata(metadata);
    }

    pub fn set_playback(&mut self, playback: MediaPlayback) {
        let _ = self.controls.set_playback(playback);
    }

    pub fn poll_events(&self) -> Vec<MediaControlEvent> {
        self.event_rx.try_iter().collect()
    }
}
