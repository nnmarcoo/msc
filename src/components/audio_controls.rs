use std::io::Cursor;

use crate::backend::track::Track;
use crate::backend::ui::{format_seconds, get_volume_color};
use crate::widgets::color_slider::color_slider;
use eframe::egui::{
    include_image, vec2, Color32, ColorImage, Context, Direction, Image, ImageButton, Layout,
    RichText, TextureHandle, TextureOptions, TopBottomPanel,
};
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
    volume_pos: f32,
    timeline_pos: f32,
    seek_pos: f32,
    _manager: AudioManager,
    sound: StreamingSoundHandle<FromFileError>,
    track: Track,
    texture_handle: TextureHandle,
}

impl AudioControls {
    pub fn new(ctx: &Context) -> Self {
        let mut manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();

        let track = Track::default();

        let bytes = include_bytes!("../../assets/setup/placeholder.mp3");
        let cursor = Cursor::new(bytes);

        let stream = StreamingSoundData::from_cursor(cursor).unwrap();
        let sound = manager.play(stream).unwrap();

        let pixels = vec![0u8; 48 * 48 * 4];

        AudioControls {
            volume_pos: 1.,
            timeline_pos: 0.,
            seek_pos: -1.,
            _manager: manager,
            sound,
            track,
            texture_handle: ctx.load_texture(
                "control_image",
                ColorImage::from_rgba_unmultiplied([48, 48], &pixels),
                TextureOptions::LINEAR,
            ),
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        let state = self.sound.state();
        let is_playing = state == PlaybackState::Playing;

        TopBottomPanel::bottom("audio_controls")
            .exact_height(80.)
            .show(ctx, |ui| {
                let icon = if is_playing {
                    include_image!("../../assets/icons/pause.png")
                } else {
                    include_image!("../../assets/icons/play.png")
                };

                ui.horizontal(|ui| {
                    ui.allocate_ui(vec2(200., ui.available_height()), |ui| {
                        ui.vertical(|ui| {
                            ui.add_space(10.);
                            ui.horizontal(|ui| {
                                ui.add(
                                    Image::new(&self.texture_handle)
                                        .max_size(vec2(48., 48.))
                                        .rounding(5.),
                                );
                                ui.vertical(|ui| {
                                    ui.add_space(10.);
                                    ui.label(RichText::from(&self.track.title).size(16.).strong());
                                    ui.label(&self.track.artist);
                                });
                            });
                        });
                    });
                    ui.allocate_ui_with_layout(
                        vec2((ui.available_width() - 150.).max(0.), ui.available_height()),
                        Layout::centered_and_justified(Direction::TopDown),
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(ui.available_width() / 2. - 20.);
                                if ui
                                    .add_sized(
                                        [25., 25.],
                                        ImageButton::new(include_image!(
                                            "../../assets/icons/previous.png"
                                        ))
                                        .rounding(100.),
                                    )
                                    .clicked()
                                {
                                    // previous song in queue
                                }

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

                                if ui
                                    .add_sized(
                                        [25., 25.],
                                        ImageButton::new(include_image!(
                                            "../../assets/icons/next.png"
                                        ))
                                        .rounding(100.),
                                    )
                                    .clicked()
                                {
                                    // next song in queue
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label(format!("{}", format_seconds(self.timeline_pos)));

                                let timeline_res = ui.add(color_slider(
                                    &mut self.timeline_pos,
                                    0.0..=self.track.duration,
                                    ui.available_width(),
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

                                if is_playing {
                                    ctx.request_repaint_after(std::time::Duration::from_millis(10));
                                    if !(timeline_res.is_pointer_button_down_on()
                                        || timeline_res.dragged())
                                        && self.seek_pos == -1.
                                    {
                                        self.timeline_pos = self.sound.position() as f32;
                                    } else if self.seek_pos.round()
                                        == self.sound.position().round() as f32
                                    {
                                        self.seek_pos = -1.;
                                        self.sound.set_volume(
                                            Volume::Amplitude(self.volume_pos as f64),
                                            Tween::default(),
                                        );
                                    }
                                }

                                ui.label(format!("{}", format_seconds(self.track.duration)));

                                ui.allocate_ui(ui.available_size(), |ui| {
                                    let volume_color = get_volume_color(self.volume_pos);

                                    if ui
                                        .add(color_slider(
                                            &mut self.volume_pos,
                                            0.0..=2.0,
                                            100.,
                                            8.,
                                            6.,
                                            volume_color,
                                        ))
                                        .changed()
                                    {
                                        self.sound.set_volume(
                                            Volume::Amplitude(self.volume_pos as f64),
                                            Tween::default(),
                                        );
                                    }
                                });
                            });
                        },
                    );
                });
            });
    }
}
