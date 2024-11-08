use crate::util::{get_audio_metadata, seconds_to_string, AudioMetadata};
use crate::{util::get_volume_color, widgets::color_slider::color_slider};
use eframe::egui::{
    include_image, vec2, Align, Color32, Context, Image, ImageButton, Layout, TextureHandle,
    TextureOptions, TopBottomPanel,
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
    manager: AudioManager,
    sound: StreamingSoundHandle<FromFileError>,
    track: AudioMetadata,
    texture_handle: Option<TextureHandle>,
}

impl AudioControls {
    pub fn new() -> Self {
        let mut manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();

        let track = get_audio_metadata("C:/bee.flac").unwrap();

        let stream = StreamingSoundData::from_file("C:/bee.flac").unwrap(); // change default audio
        let sound = manager.play(stream).unwrap();

        AudioControls {
            volume_pos: 1.,
            timeline_pos: 0.,
            seek_pos: -1.,
            manager,
            sound,
            track,
            texture_handle: None,
        }
    }

    pub fn show(&mut self, ctx: &Context) {

        if self.texture_handle.is_none() {
            self.texture_handle = Some(ctx.load_texture(
                "art",
                self.track.image.clone(),
                TextureOptions::default(),
            ));
        }

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
                    if let Some(handle) = &self.texture_handle {
                        let image = Image::new(handle).rounding(2.).max_size(vec2(60., 60.));
                        ui.add(image);
                    }

                    ui.label(&self.track.title);
                    ui.label(&self.track.artist);
                    

                    if ui
                        .add_sized(
                            [25., 25.],
                            ImageButton::new(include_image!("../../assets/icons/previous.png"))
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
                            ImageButton::new(include_image!("../../assets/icons/next.png"))
                                .rounding(100.),
                        )
                        .clicked()
                    {
                        // next song in queue
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(format!("{}", seconds_to_string(self.timeline_pos)));

                    let timeline_res = ui.add(color_slider(
                        &mut self.timeline_pos,
                        0.0..=self.track.duration,
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

                    if is_playing {
                        ctx.request_repaint_after(std::time::Duration::from_millis(10));
                        if !(timeline_res.is_pointer_button_down_on() || timeline_res.dragged())
                            && self.seek_pos == -1.
                        {
                            self.timeline_pos = self.sound.position() as f32;
                        } else if self.seek_pos.round() == self.sound.position().round() as f32 {
                            self.seek_pos = -1.;
                            self.sound.set_volume(
                                Volume::Amplitude(self.volume_pos as f64),
                                Tween::default(),
                            );
                        }
                    }

                    ui.label(format!("{}", seconds_to_string(self.track.duration)));

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
    }
}
