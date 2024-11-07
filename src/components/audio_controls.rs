use std::io::Cursor;
use std::time::Duration;

use crate::{util::get_volume_color, widgets::color_slider::color_slider};
use eframe::egui::{include_image, Color32, Context, ImageButton, TopBottomPanel};
use kira::tween::Tween;
use kira::Volume;
use kira::{
    manager::{AudioManager, AudioManagerSettings, DefaultBackend},
    sound::{
        streaming::{StreamingSoundData, StreamingSoundHandle},
        FromFileError, PlaybackState,
    },
};

pub struct AudioControls {
    is_playing: bool,
    volume: f32,
    handle_pos: f32,
    is_timeline_dragged: bool,
    manager: AudioManager,
    sound: StreamingSoundHandle<FromFileError>,
    duration: f32,
}

impl AudioControls {
    pub fn new() -> Self {
        let mut manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();

        let stream = StreamingSoundData::from_file("C:/hers.mp3").unwrap(); // change default audio
        let duration = stream.duration().as_secs_f32();
        let sound = manager.play(stream).unwrap();

        AudioControls {
            is_playing: true, // change
            volume: 1.,
            handle_pos: 0.,
            is_timeline_dragged: false,
            manager,
            sound,
            duration,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        if self.is_playing {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            if !self.is_timeline_dragged {
                self.handle_pos = self.sound.position() as f32;
            }
        }

        TopBottomPanel::bottom("audio_controls")
            .exact_height(80.)
            .show(ctx, |ui| {
                if self.is_playing {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    if !self.is_timeline_dragged {
                        self.handle_pos = self.sound.position() as f32;
                    }
                }


                let icon = if self.is_playing {
                    include_image!("../../assets/icons/pause.png")
                } else {
                    include_image!("../../assets/icons/play.png")
                };

                ui.horizontal(|ui| {
                    if ui
                        .add_sized([30., 30.], ImageButton::new(icon).rounding(100.))
                        .clicked()
                    {
                        if self.is_playing {
                            self.sound.pause(Tween::default());
                        } else {
                            self.sound.resume(Tween::default());
                        }
                    }

                    ui.label(format!("{:.1}", self.sound.position()));

                    let timeline_res = ui.add(color_slider(
                        &mut self.handle_pos,
                        0.0..=self.duration,
                        200.,
                        8.,
                        6.,
                        Color32::from_rgb(0, 92, 128),
                    ));

                    if timeline_res.drag_stopped() || timeline_res.clicked() {
                        self.sound.seek_to(self.handle_pos as f64);
                    }

                    if timeline_res.is_pointer_button_down_on() || timeline_res.dragged() {
                        self.is_timeline_dragged = true;
                    } else {
                        self.is_timeline_dragged = false;
                    }

                    

                    let volume_color = get_volume_color(self.volume);

                    if ui
                        .add(color_slider(
                            &mut self.volume,
                            0.0..=2.0,
                            100.,
                            8.,
                            6.,
                            volume_color,
                        ))
                        .changed()
                    {
                        self.sound
                            .set_volume(Volume::Amplitude(self.volume as f64), Tween::default());
                    }
                });
            });
    }
}
