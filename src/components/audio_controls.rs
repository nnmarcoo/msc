use crate::{util::get_volume_color, widgets::color_slider::color_slider};
use eframe::egui::{include_image, Color32, Context, ImageButton, TopBottomPanel};

pub struct AudioControls {
    is_playing: bool,
    volume: f32,
    handle_pos: f32,
    is_timeline_dragged: bool,
}

impl AudioControls {
    pub fn new() -> Self {
        AudioControls {
            is_playing: false,
            volume: 1.,
            handle_pos: 0.,
            is_timeline_dragged: false,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        if self.is_playing {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            if !self.is_timeline_dragged {
                //self.handle_pos = self.sink.get_pos().as_secs_f32();
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

                ui.horizontal(|ui| {
                    if ui
                        .add_sized(
                            [30., 30.],
                            ImageButton::new(include_image!("../../assets/icons/x.png"))
                                .rounding(100.),
                        )
                        .clicked()
                    {
                        //self.toggle_playback();
                    }

                    if ui
                        .add_sized([30., 30.], ImageButton::new(icon).rounding(100.))
                        .clicked()
                    {
                        //self.toggle_playback();
                    }

                    if ui
                        .add_sized(
                            [30., 30.],
                            ImageButton::new(include_image!("../../assets/icons/x.png"))
                                .rounding(100.),
                        )
                        .clicked()
                    {
                        //self.toggle_playback();
                    }
                });

                ui.horizontal(|ui| {
                    //ui.label(seconds_to_string(self.handle_pos));

                    let timeline_res = ui.add(color_slider(
                        &mut self.handle_pos,
                        0.0..=1.0, //
                        200.,
                        8.,
                        6.,
                        Color32::from_rgb(0, 92, 128),
                    ));

                    if timeline_res.drag_stopped() || timeline_res.clicked() {
                        //self.sink.try_seek(Duration::new(self.handle_pos as u64, 0)).unwrap();
                    }

                    if timeline_res.is_pointer_button_down_on() || timeline_res.dragged() {
                        self.is_timeline_dragged = true;
                    } else {
                        self.is_timeline_dragged = false;
                    }

                    //ui.label(duration_to_string(self.track_length));

                    let volume_color = get_volume_color(1.);

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
                        //self.sink.set_volume(self.volume);
                    }
                });
            });
    }
}
