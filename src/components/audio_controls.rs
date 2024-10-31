use eframe::egui::{include_image, Context, ImageButton, Slider, SliderClamping, TopBottomPanel};
use rodio::{Decoder, OutputStream, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

use crate::util::duration_to_string;

pub struct AudioControls {
    is_playing: bool,
    volume: f32,
    sink: Sink,
    track_length: Duration,
    _stream: OutputStream,
}

impl AudioControls {
    pub fn new() -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        let audio_file_path = "C:/hers.mp3";
        let file = BufReader::new(File::open(audio_file_path).unwrap());

        let source = Decoder::new(file).unwrap();
        let duration = source.total_duration().unwrap_or(Duration::new(0, 0));

        sink.append(source);
        sink.pause();

        AudioControls {
            is_playing: false,
            volume: 1.0,
            sink,
            _stream: stream,
            track_length: duration,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        if self.is_playing {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        TopBottomPanel::bottom("audio_controls")
            .exact_height(80.0)
            .show(ctx, |ui| {
                let icon = if self.is_playing {
                    include_image!("../../assets/icons/pause.png")
                } else {
                    include_image!("../../assets/icons/play.png")
                };

                if ui
                    .add_sized([40.0, 40.0], ImageButton::new(icon).rounding(100.))
                    .clicked()
                {
                    self.toggle_playback();
                }

                if ui
                    .add(
                        Slider::new(&mut self.volume, 0.0..=2.0)
                            .clamping(SliderClamping::Always)
                            .show_value(false),
                    )
                    .changed()
                {
                    self.sink.set_volume(self.volume);
                }

                ui.horizontal(|ui| {
                    ui.label(duration_to_string(self.sink.get_pos()));
                    ui.label(duration_to_string(self.track_length));
                });
            });
    }

    pub fn toggle_playback(&mut self) {
        if self.is_playing {
            self.sink.pause();
        } else {
            self.sink.play();
        }
        self.is_playing = !self.is_playing;
    }
}
