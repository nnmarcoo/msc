use crate::{
    core::helps::format_seconds,
    structs::{State, View},
    widgets::{color_slider::color_slider, styled_button::StyledButton},
};
use eframe::egui::TopBottomPanel;
use egui::{include_image, vec2, Align, Color32, Context, Image, Label, Layout, RichText};

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct AudioControls {
    timeline_pos: f32,
    seek_pos: f32,
    volume: f32,
}

impl AudioControls {
    pub fn new() -> Self {
        AudioControls {
            timeline_pos: 0.,
            seek_pos: -1.,
            volume: 0.5,
        }
    }

    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        let is_playing = state.queue.is_playing();

        TopBottomPanel::bottom("audio_controls")
            .exact_height(64.)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add(
                        StyledButton::new(
                            vec2(22., 22.),
                            &Image::new(include_image!("../../assets/icons/previous.png")),
                            || {},
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
                            || {},
                        )
                        .with_hover_text("Next")
                        .with_rounding(5.),
                    );

                    ui.add_space(15.);

                    let (vol_icon, vol_text) = if self.volume > 0. {
                        (include_image!("../../assets/icons/vol_on.png"), "Mute")
                    } else {
                        (include_image!("../../assets/icons/vol_off.png"), "Unmute")
                    };

                    ui.add(
                        StyledButton::new(vec2(22., 22.), &Image::new(vol_icon), || {
                            self.volume = if self.volume > 0. { 0. } else { 0.5 };
                            state.queue.set_volume(self.volume);
                        })
                        .with_hover_text(vol_text)
                        .with_rounding(5.),
                    );

                    let _ = ui.add(color_slider(
                        &mut self.volume,
                        0.0..=1.,
                        70.,
                        4.,
                        4.,
                        Color32::from_rgb(0, 92, 128),
                    ));

                    ui.add_space(10.);

                    ui.allocate_ui(
                        vec2((ui.available_width() - 250.).max(0.), ui.available_height()),
                        |ui| {
                            ui.vertical(|ui| {
                                ui.add_space(20.);
                                ui.vertical(|ui| {
                                    ui.with_layout(Layout::left_to_right(Align::LEFT), |ui| {
                                        ui.add(
                                            Label::new(RichText::new("Title").strong()).truncate(),
                                        );
                                        ui.add(Label::new("Artist").truncate());
                                        ui.add_space(ui.available_width());

                                        let duration = format_seconds(0.);

                                        ui.label(format!(
                                            "{} / {}",
                                            format_seconds(self.timeline_pos),
                                            duration
                                        ));
                                    });

                                    ui.add_space(1.);

                                    let timeline_res = ui.add(color_slider(
                                        &mut self.timeline_pos,
                                        0.0..=0.,
                                        ui.available_width(),
                                        4.,
                                        4.,
                                        Color32::from_rgb(0, 92, 128),
                                    ));

                                    if timeline_res.drag_stopped() || timeline_res.clicked() {
                                        self.seek_pos = self.timeline_pos;
                                    }
                                });
                            });
                        },
                    );

                    ui.add_space(30.);

                    if ui.button("🔀").clicked() {}
                    if ui.button("⟲").clicked() {}
                    if ui.button("🔜").clicked() {}
                    if ui.button("⛭").clicked() {
                        state.view = View::Settings;
                    }
                });
            });
    }
}
