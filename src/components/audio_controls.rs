use crate::util::seconds_to_string;
use crate::{util::get_volume_color, widgets::color_slider::color_slider};
use eframe::egui::{include_image, Color32, Context, ImageButton, TopBottomPanel};
use kira::sound::PlaybackState;
use kira::tween::Tween;
use kira::Volume;
use kira::{
    manager::{AudioManager, AudioManagerSettings, DefaultBackend},
    sound::{
        streaming::{StreamingSoundData, StreamingSoundHandle},
        FromFileError,
    },
};

pub struct AudioControls {
    volume: f32,
    timeline_pos: f32,
    seek_pos: f32,
    is_timeline_dragged: bool,
    manager: AudioManager,
    sound: StreamingSoundHandle<FromFileError>,
    duration: f32,
}

impl AudioControls {
    pub fn new() -> Self {
        let mut manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();

        let stream = StreamingSoundData::from_file("C:/bee.flac").unwrap(); // change default audio
        let duration = stream.duration().as_secs_f32();
        let sound = manager.play(stream).unwrap();

        AudioControls {
            volume: 1.,
            timeline_pos: 0.,
            seek_pos: -1.,
            is_timeline_dragged: false,
            manager,
            sound,
            duration,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        let state = self.sound.state();
        let is_playing = state == PlaybackState::Playing;

        if is_playing {
            ctx.request_repaint_after(std::time::Duration::from_millis(10));
            if !self.is_timeline_dragged && self.seek_pos == -1. {
                self.timeline_pos = self.sound.position() as f32;
            } else if self.seek_pos.round() == self.sound.position().round() as f32 {
                self.seek_pos = -1.;
                self.sound
                    .set_volume(Volume::Amplitude(self.volume as f64), Tween::default());
            }
        }

        TopBottomPanel::bottom("audio_controls")
            .exact_height(80.)
            .show(ctx, |ui| {
                let icon = if is_playing {
                    include_image!("../../assets/icons/pause.png")
                } else {
                    include_image!("../../assets/icons/play.png")
                };

                if ui
                    .add_sized([30., 30.], ImageButton::new(icon).rounding(100.))
                    .clicked()
                {
                    if is_playing {
                        self.sound.pause(Tween::default());
                    } else {
                        self.sound.resume(Tween::default());
                    }
                }

                ui.horizontal(|ui| {
                    ui.label(format!("{}", seconds_to_string(self.timeline_pos)));

                    let timeline_res = ui.add(color_slider(
                        &mut self.timeline_pos,
                        0.0..=self.duration,
                        ui.available_width() - 150.,
                        8.,
                        6.,
                        Color32::from_rgb(0, 92, 128),
                    ));

                    if timeline_res.drag_stopped() || timeline_res.clicked() {
                        self.seek_pos = self.timeline_pos;
                        self.sound
                            .set_volume(Volume::Amplitude(0.), Tween::default());
                        self.sound.seek_to(self.timeline_pos as f64);
                    }

                    if timeline_res.is_pointer_button_down_on() || timeline_res.dragged() {
                        self.is_timeline_dragged = true;
                    } else {
                        self.is_timeline_dragged = false;
                    }

                    ui.label(format!("{}", seconds_to_string(self.duration)));

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
