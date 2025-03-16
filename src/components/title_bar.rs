use eframe::egui::{
    include_image,
    menu::{self},
    vec2, Align, Color32, Context, Frame, Layout, Margin, PointerButton, Sense, TextEdit,
    TopBottomPanel, ViewportCommand,
};
use egui::Image;
use serde::{Deserialize, Serialize};

use crate::{structs::WindowState, widgets::button::Button};

#[derive(Serialize, Deserialize, Default)]
pub struct TitleBar {
    pub window_state: WindowState,
    #[serde(skip)]
    pub query: String,
    bar_height: f32,
}

impl TitleBar {
    pub fn new() -> Self {
        TitleBar {
            window_state: WindowState::default(),
            query: String::new(),
            bar_height: 32.,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        TopBottomPanel::top("title_bar")
            .frame(Frame::default().inner_margin(Margin::ZERO))
            .show(ctx, |ui| {
                self.window_state.is_maximized =
                    ui.input(|i| i.viewport().maximized.unwrap_or(false));

                let res = ui.interact(ui.max_rect(), ui.id(), Sense::click_and_drag());

                if res.drag_started_by(PointerButton::Primary) {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                    self.window_state.is_dragging = true;
                }

                if res.drag_stopped() {
                    self.window_state.is_dragging = false;
                }

                if res.double_clicked_by(PointerButton::Primary) {
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(
                        !self.window_state.is_maximized,
                    ));
                }

                menu::bar(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // x
                        ui.add(
                            Button::new(
                                vec2(self.bar_height, self.bar_height),
                                &Image::new(include_image!("../../assets/icons/x.png")),
                                || ctx.send_viewport_cmd(ViewportCommand::Close),
                            )
                            .with_hover_color(Color32::from_rgb(232, 17, 35)),
                        );

                        // maximize/restore
                        if self.window_state.is_maximized {
                            ui.add(Button::new(
                                vec2(self.bar_height, self.bar_height),
                                &Image::new(include_image!("../../assets/icons/restore.png")),
                                || ctx.send_viewport_cmd(ViewportCommand::Maximized(false)),
                            ));
                        } else {
                            ui.add(Button::new(
                                vec2(self.bar_height, self.bar_height),
                                &Image::new(include_image!("../../assets/icons/maximize.png")),
                                || ctx.send_viewport_cmd(ViewportCommand::Maximized(true)),
                            ));
                        }

                        // minimize
                        ui.add(Button::new(
                            vec2(self.bar_height, self.bar_height),
                            &Image::new(include_image!("../../assets/icons/minimize.png")),
                            || ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
                        ));

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

                        // setttings
                        ui.allocate_ui(vec2(28., 28.), |ui| {
                            ui.menu_image_button(
                                include_image!("../../assets/icons/settings.png"),
                                |ui| {
                                    if ui.button("About").clicked() {}
                                    if ui.button("Help").clicked() {}
                                    if ui.button("Update").clicked() {}
                                    ui.separator();
                                    if ui.button("Settings").clicked() {
                                        //state.view = View::Settings;
                                        ui.close_menu();
                                    }
                                },
                            );
                        });
                    });
                });
            });
    }
}
