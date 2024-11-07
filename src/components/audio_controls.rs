use std::io::Cursor;

use crate::{util::get_volume_color, widgets::color_slider::color_slider};
use eframe::egui::{include_image, Color32, Context, ImageButton, TopBottomPanel};
use kira::{manager::{AudioManager, AudioManagerSettings, DefaultBackend}, sound::{streaming::{StreamingSoundData, StreamingSoundHandle}, FromFileError, PlaybackState}};
use kira::tween::Tween;
use kira::Volume;

pub struct AudioControls {
    is_playing: bool,
    volume: f32,
    handle_pos: f32,
    is_timeline_dragged: bool,
    manager: AudioManager,
    sound: StreamingSoundHandle<FromFileError>,
}

impl AudioControls {
    pub fn new() -> Self {
        let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();

        let silent_audio_data: Vec<u8> = vec![0; 44100 * 2 * 2]; // 1 second of silence for 16-bit stereo at 44.1 kHz
        let cursor = Cursor::new(silent_audio_data);
        let stream = StreamingSoundData::from_cursor(cursor).unwrap();
        let sound = manager.play(stream).unwrap();

        AudioControls {
            is_playing: false,
            volume: 1.,
            handle_pos: 0.,
            is_timeline_dragged: false,
            manager,
            sound,
        }
    }

    pub fn show(&mut self, ctx: &Context) -> Result<(), &'static str> {
        if self.is_playing {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            if !self.is_timeline_dragged {
                //self.handle_pos = self.get_sound()?.position() as f32;
            }
        }

        TopBottomPanel::bottom("audio_controls")
            .exact_height(80.)
            .show(ctx, |ui| {
                let icon = if self.is_playing {
                    include_image!("../../assets/icons/pause.png")
                } else {
                    include_image!("../../assets/icons/play.png")
                };

                ui.horizontal(|ui|  {

                    if ui
                        .add_sized([30., 30.], ImageButton::new(icon).rounding(100.))
                        .clicked()
                    {
                        //self.toggle_playback();
                    }
                });

            });
            Ok(())
    }
}
