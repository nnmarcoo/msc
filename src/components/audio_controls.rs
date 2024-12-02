use std::time::Duration;

use crate::backend::ui::{format_seconds, get_volume_color};
use crate::msc::State;
use crate::widgets::color_slider::color_slider;
use eframe::egui::{
    include_image, vec2, Color32, Context, Direction, Image, ImageButton, Layout, RichText,
    TopBottomPanel,
};

pub struct AudioControls {
    timeline_pos: f32,
    seek_pos: f32,
}

impl AudioControls {
    pub fn new() -> Self {
        AudioControls {
            timeline_pos: 0.,
            seek_pos: -1.,
        }
    }

    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        let is_playing = state.queue.is_playing();

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
                                if state.config.show_image {
                                    if let Some(track) = state.queue.current_track() {
                                        track.load_texture(ctx);

                                        if let Some(texture) = &track.texture {
                                            ui.add(
                                                Image::new(texture)
                                                    .max_size(vec2(48., 48.))
                                                    .rounding(5.),
                                            );
                                        }
                                    }
                                }
                                ui.vertical(|ui| {
                                    ui.add_space(10.);
                                    ui.label(
                                        RichText::from(&state.queue.current_track().unwrap().title)
                                            .size(16.)
                                            .strong(),
                                    );
                                    ui.label(&state.queue.current_track().unwrap().artist);
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
                                    state.queue.play_previous_track(state.config.volume as f64);
                                }

                                if ui
                                    .add_sized([30., 30.], ImageButton::new(icon).rounding(100.))
                                    .clicked()
                                {
                                    state.queue.toggle_playback();
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
                                    state.queue.play_next_track(state.config.volume as f64);
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label(format!("{}", format_seconds(self.timeline_pos)));

                                let timeline_res = ui.add(color_slider(
                                    &mut self.timeline_pos,
                                    0.0..=state.queue.current_track().unwrap().duration,
                                    ui.available_width(),
                                    8.,
                                    6.,
                                    Color32::from_rgb(0, 92, 128),
                                ));

                                if timeline_res.drag_stopped() || timeline_res.clicked() {
                                    self.seek_pos = self.timeline_pos;
                                    state.queue.set_volume(0.);
                                    state.queue.seek(self.timeline_pos);
                                }

                                if is_playing {
                                    if state.config.redraw {
                                        ctx.request_repaint_after(Duration::from_secs_f32(
                                            state.config.redraw_time,
                                        ));
                                    }
                                    if !(timeline_res.is_pointer_button_down_on()
                                        || timeline_res.dragged())
                                        && self.seek_pos == -1.
                                    {
                                        self.timeline_pos = state.queue.position();
                                    } else if self.seek_pos.round()
                                        == state.queue.position().round()
                                    {
                                        self.seek_pos = -1.;
                                        state.queue.set_volume(state.config.volume);
                                    }
                                }

                                ui.label(format!(
                                    "{}",
                                    format_seconds(state.queue.current_track().unwrap().duration)
                                ));

                                ui.allocate_ui(ui.available_size(), |ui| {
                                    let volume_color = get_volume_color(state.config.volume);

                                    let volume_slider = ui.add(color_slider(
                                        &mut state.config.volume,
                                        0.0..=2.0,
                                        100.,
                                        8.,
                                        6.,
                                        volume_color,
                                    ));

                                    if volume_slider.double_clicked() {
                                        state.config.volume = 1.;
                                        state.queue.set_volume(state.config.volume);
                                    }

                                    if volume_slider.changed() {
                                        state.queue.set_volume(state.config.volume);
                                    }
                                });
                            });
                        },
                    );
                });
            });
    }
}
