use crate::{core::helps::format_seconds, widgets::color_slider::color_slider};
use eframe::egui::TopBottomPanel;
use egui::{vec2, Align, Color32, Context, Layout};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
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

    pub fn show(&mut self, ctx: &Context) {
        TopBottomPanel::bottom("audio_controls")
            .exact_height(64.)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let _ = ui.button("⏴");
                    let _ = ui.button("⏸");
                    let _ = ui.button("⏵");

                    ui.add_space(10.);

                    let _ = ui.button("🔊");
                    let _ = ui.add(color_slider(
                        &mut self.volume,
                        0.0..=100.,
                        70.,
                        4.,
                        4.,
                        Color32::from_rgb(0, 92, 128),
                    ));

                    ui.add_space(10.);

                    ui.allocate_ui(
                        vec2((ui.available_width() - 300.).max(0.), ui.available_height()),
                        |ui| {
                            ui.vertical(|ui| {
                                ui.add_space(20.);
                                ui.vertical(|ui| {
                                    ui.with_layout(Layout::left_to_right(Align::LEFT), |ui| {
                                        ui.strong("Title");
                                        ui.label("Artist");
                                        ui.add_space(ui.available_width());

                                        let duration = format_seconds(100.);

                                        ui.label(format!(
                                            "{} / {}",
                                            format_seconds(self.timeline_pos),
                                            duration
                                        ));
                                    });

                                    ui.add_space(1.);

                                    let timeline_res = ui.add(color_slider(
                                        &mut self.timeline_pos,
                                        0.0..=100.,
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

                    ui.add_space(60.);

                    let _ = ui.button("🔀"); // shuffle queue
                    let _ = ui.button("⟲"); // repeat
                    let _ = ui.button("🔜"); // queue
                    let _ = ui.button("⛭"); // settings
                });
            });
    }
}
