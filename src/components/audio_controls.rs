use std::time::Duration;

use crate::{
    core::helps::format_seconds,
    state::{State, View},
    widgets::{color_slider::color_slider, styled_button::StyledButton},
};
use eframe::egui::TopBottomPanel;
use egui::{include_image, vec2, Align, Color32, Context, Image, Label, Layout, RichText};

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct AudioControls {
    seek_pos: f32,
}

impl AudioControls {
    pub fn new() -> Self {
        AudioControls { seek_pos: -1. }
    }

    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        let current_track = state.queue.get_current_track();
        let is_playing = state.queue.is_playing();

        TopBottomPanel::bottom("audio_controls")
            .exact_height(64.)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add(
                        StyledButton::new(
                            vec2(22., 22.),
                            &Image::new(include_image!("../../assets/icons/previous.png")),
                            || {
                                state.queue.play_previous();
                            },
                        )
                        .with_hover_text("Previous")
                        .with_rounding(5.),
                    );

                    let (playback_icon, playback_text) = if is_playing {
                        (include_image!("../../assets/icons/pause.png"), "Pause")
                    } else {
                        (include_image!("../../assets/icons/play.png"), "Play")
                    };

                    ui.add(
                        StyledButton::new(vec2(28., 28.), &Image::new(playback_icon), || {
                            state.queue.toggle_playback();
                        })
                        .with_hover_text(playback_text)
                        .with_rounding(5.),
                    );
                    ui.add(
                        StyledButton::new(
                            vec2(22., 22.),
                            &Image::new(include_image!("../../assets/icons/next.png")),
                            || {
                                state.queue.play_next();
                            },
                        )
                        .with_hover_text("Next")
                        .with_rounding(5.),
                    );

                    ui.add_space(15.);

                    let (vol_icon, vol_text) = if state.queue.volume > 0. {
                        (include_image!("../../assets/icons/vol_on.png"), "Mute")
                    } else {
                        (include_image!("../../assets/icons/vol_off.png"), "Unmute")
                    };

                    ui.add(
                        StyledButton::new(vec2(22., 22.), &Image::new(vol_icon), || {
                            state.queue.volume = if state.queue.volume > 0. { 0. } else { 0.5 };
                            state.queue.update_volume();
                        })
                        .with_hover_text(vol_text)
                        .with_rounding(5.),
                    );

                    if ui
                        .add(color_slider(
                            &mut state.queue.volume,
                            0.0..=1.,
                            70.,
                            4.,
                            4.,
                            Color32::from_rgb(0, 92, 128),
                        ))
                        .changed()
                    {
                        state.queue.update_volume();
                    }

                    ui.add_space(10.);

                    ui.allocate_ui(
                        vec2((ui.available_width() - 250.).max(0.), ui.available_height()),
                        |ui| {
                            ui.vertical(|ui| {
                                ui.add_space(20.);
                                ui.vertical(|ui| {
                                    ui.with_layout(Layout::left_to_right(Align::LEFT), |ui| {
                                        ui.add(
                                            Label::new(RichText::new(current_track.title).strong())
                                                .truncate(),
                                        );
                                        ui.add(Label::new(current_track.artist).truncate());
                                        ui.add_space(ui.available_width());

                                        let duration = format_seconds(current_track.duration);

                                        ui.label(format!(
                                            "{} / {}",
                                            format_seconds(state.queue.timeline_pos),
                                            duration
                                        ));
                                    });

                                    ui.add_space(1.);

                                    let timeline_res = ui.add(color_slider(
                                        &mut state.queue.timeline_pos,
                                        0.0..=current_track.duration,
                                        ui.available_width(),
                                        4.,
                                        4.,
                                        Color32::from_rgb(0, 92, 128),
                                    ));

                                    if timeline_res.drag_stopped() || timeline_res.clicked() {
                                        self.seek_pos = state.queue.timeline_pos;
                                        state.queue.seek(state.queue.timeline_pos);
                                    }

                                    if is_playing {
                                        // bad
                                        ctx.request_repaint_after(Duration::from_millis(200));

                                        if !(timeline_res.is_pointer_button_down_on()
                                            || timeline_res.dragged())
                                            && self.seek_pos == -1.
                                        {
                                            state.queue.timeline_pos = state.queue.position();
                                        } else if self.seek_pos.floor()
                                            == state.queue.position().floor()
                                        {
                                            self.seek_pos = -1.;
                                        }
                                    }
                                });
                            });
                        },
                    );

                    ui.add_space(30.);

                    if ui.button("🔀").clicked() {
                        state.queue.clear();
                    }
                    if ui.button("⟲").clicked() {}
                    if ui.button("🔜").clicked() {}
                    if ui.button("⛭").clicked() {
                        state.view = View::Settings;
                    }
                });
            });
    }
}
