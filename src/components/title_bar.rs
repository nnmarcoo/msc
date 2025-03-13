use eframe::egui::{
    include_image,
    menu::{self},
    vec2, Align, Color32, Context, Frame, ImageButton, Layout, Margin, PointerButton, Sense,
    TextEdit, TopBottomPanel, ViewportCommand,
};
use serde::{Deserialize, Serialize};

use crate::structs::WindowState;

#[derive(Serialize, Deserialize, Default)]
pub struct TitleBar {
    pub window_state: WindowState,
    pub query: String,
    bar_width: f32,
}

impl TitleBar {
    pub fn new() -> Self {
        TitleBar {
            window_state: WindowState::default(),
            query: String::new(),
            bar_width: 30.,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        TopBottomPanel::top("title_bar")
            .frame(Frame::default().inner_margin(Margin::ZERO))
            .show(ctx, |ui| {
                self.window_state.is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));

                let res = ui.interact(ui.max_rect(), ui.id(), Sense::click_and_drag());

                if res.drag_started_by(PointerButton::Primary) {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                    self.window_state.is_dragging = true;
                }

                if res.drag_stopped() {
                    self.window_state.is_dragging = false;
                }

                if res.double_clicked_by(PointerButton::Primary) {
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(!self.window_state.is_maximized));
                }

                menu::bar(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.scope(|ui| {
                            ui.style_mut().visuals.widgets.hovered.weak_bg_fill =
                                Color32::from_rgb(232, 17, 35);
                            if ui
                                .add_sized(
                                    [self.bar_width, self.bar_width],
                                    ImageButton::new(include_image!("../../assets/icon-256.png")),
                                )
                                .clicked()
                            {
                                ctx.send_viewport_cmd(ViewportCommand::Close);
                            }
                        });
                
                        if self.window_state.is_maximized {
                            if ui
                                .add_sized(
                                    [self.bar_width, self.bar_width],
                                    ImageButton::new(include_image!("../../assets/icon-256.png")),
                                )
                                .clicked()
                            {
                                ctx.send_viewport_cmd(ViewportCommand::Maximized(false));
                            }
                        } else {
                            if ui
                                .add_sized(
                                    [self.bar_width, self.bar_width],
                                    ImageButton::new(include_image!("../../assets/icon-256.png")),
                                )
                                .clicked()
                            {
                                ctx.send_viewport_cmd(ViewportCommand::Maximized(true));
                            }
                        }
                
                        if ui
                            .add_sized(
                                [self.bar_width, self.bar_width],
                                ImageButton::new(include_image!("../../assets/icon-256.png")),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                        }

                        ui.add_space(5.);
                
                        if ui
                            .add(
                                TextEdit::singleline(&mut self.query)
                                    .hint_text("🔍 Search a song")
                                    .desired_width(150.),
                            )
                            .has_focus()
                        {
                            //state.view = View::Library;
                        }
                
                        ui.add_space(ui.available_width() - 47.);
                
                        ui.allocate_ui(vec2(28., 28.), |ui| {
                            ui.menu_image_button(include_image!("../../assets/icon-256.png"), |ui| {
                                if ui.button("About").clicked() {}
                                if ui.button("Help").clicked() {}
                                if ui.button("Update").clicked() {}
                                ui.separator();
                                if ui.button("Settings").clicked() {
                                    //state.view = View::Settings;
                                    ui.close_menu();
                                }
                            });
                        });
                    });
                });
                
            });
    }
}