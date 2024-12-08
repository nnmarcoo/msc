use std::sync::Arc;
use std::time::Duration;

use crate::backend::ui::{format_seconds, get_volume_color};
use crate::constants::DEFAULT_IMAGE_BORDER_IMAGE;
use crate::msc::State;
use crate::widgets::color_slider::color_slider;
use eframe::egui::{
    include_image, vec2, Align, Color32, Context, Image, ImageButton, Label, Layout, RichText,
    TextWrapMode, TopBottomPanel,
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
                let playback_icon = if is_playing {
                    include_image!("../../assets/icons/pause.png")
                } else {
                    include_image!("../../assets/icons/play.png")
                };

                let volume_icon = if state.config.volume > 0. {
                    include_image!("../../assets/icons/volume.png")
                } else {
                    include_image!("../../assets/icons/volumeoff.png")
                };

                ui.horizontal_centered(|ui| {
                    if ui
                        .add_sized(
                            [25., 25.],
                            ImageButton::new(include_image!("../../assets/icons/previous.png"))
                                .rounding(5.),
                        )
                        .clicked()
                    {
                        state.queue.play_previous_track(state.config.volume as f64);
                    }

                    if ui
                        .add_sized([30., 30.], ImageButton::new(playback_icon).rounding(5.))
                        .clicked()
                    {
                        state.queue.toggle_playback();
                    }

                    if ui
                        .add_sized(
                            [25., 25.],
                            ImageButton::new(include_image!("../../assets/icons/next.png"))
                                .rounding(5.),
                        )
                        .clicked()
                    {
                        state.queue.play_next_track(state.config.volume as f64);
                    }

                    ui.allocate_ui(
                        vec2(ui.available_width() - 370., ui.available_height()),
                        |ui| {
                            ui.vertical(|ui| {
                                ui.add_space(26.);
                                ui.vertical(|ui| {
                                    ui.with_layout(Layout::left_to_right(Align::LEFT), |ui| {
                                        let track = state.queue.current_track().unwrap().clone();
                                        ui.strong(track.title);
                                        ui.label(track.artist);
                                        ui.add_space(ui.available_width());

                                        let duration = if state.config.show_duration {
                                            format_seconds(
                                                state.queue.current_track().unwrap().duration,
                                            )
                                        } else {
                                            format!(
                                                "-{}",
                                                format_seconds(
                                                    state
                                                        .queue
                                                        .current_track()
                                                        .unwrap()
                                                        .duration
                                                        .floor()
                                                        - self.timeline_pos.floor()
                                                )
                                            )
                                        };

                                        if ui
                                            .label(format!(
                                                "{} / {}",
                                                format_seconds(self.timeline_pos),
                                                duration
                                            ))
                                            .clicked()
                                        {
                                            state.config.show_duration =
                                                !state.config.show_duration;
                                        }
                                    });

                                    ui.add_space(1.);

                                    let timeline_res = ui.add(color_slider(
                                        &mut self.timeline_pos,
                                        0.0..=state.queue.current_track().unwrap().duration,
                                        ui.available_width(),
                                        4.,
                                        4.,
                                        Color32::from_rgb(0, 92, 128),
                                    ));

                                    if timeline_res.drag_stopped() || timeline_res.clicked() {
                                        self.seek_pos = self.timeline_pos;
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
                                        } else if self.seek_pos.floor()
                                            == state.queue.position().floor()
                                        {
                                            self.seek_pos = -1.;
                                            state.queue.set_volume(state.config.volume);
                                        }
                                    }
                                });
                            });
                        },
                    );

                    ui.add_space(5.);

                    if ui
                        .add_sized([25., 25.], ImageButton::new(volume_icon).rounding(5.))
                        .clicked()
                    {
                        // change this awfulness
                        if state.config.volume > 0. {
                            state.config.volume = 0.;
                            state.queue.set_volume(0.);
                        } else {
                            state.config.volume = 1.;
                            state.queue.set_volume(1.);
                        }
                    }

                    let volume_color = get_volume_color(state.config.volume);

                    let volume_slider = ui.add(color_slider(
                        &mut state.config.volume,
                        0.0..=2.0,
                        100.,
                        4.,
                        4.,
                        volume_color,
                    ));

                    if volume_slider.double_clicked() {
                        state.config.volume = 1.;
                        state.queue.set_volume(state.config.volume);
                    }

                    if volume_slider.changed() {
                        state.queue.set_volume(state.config.volume);
                    }

                    ui.add_space(5.);

                    /*
                    if state.config.show_image {
                        if let Some(track) = state.queue.current_track() {
                            track.load_texture_async(ctx.clone(), Arc::clone(&state.image_loader));

                            let image = match &track.get_texture() {
                                Some(texture) => Image::new(texture),
                                None => Image::new(DEFAULT_IMAGE_BORDER_IMAGE),
                            };

                            ui.add_sized([48., 48.], image.max_size(vec2(48., 48.)).rounding(5.));
                        }
                    }

                    ui.vertical(|ui| {
                        ui.add_space(20.);
                        ui.add(
                            Label::new(
                                RichText::from(&state.queue.current_track().unwrap().title)
                                    .size(16.)
                                    .strong(),
                            )
                            .wrap_mode(TextWrapMode::Truncate),
                        );

                        ui.add(
                            Label::new(RichText::from(
                                &state.queue.current_track().unwrap().artist,
                            ))
                            .wrap_mode(TextWrapMode::Truncate),
                        );
                    });
                     */
                });
            });
    }
}
