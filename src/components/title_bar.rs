use eframe::egui::{
    include_image,
    menu::{self},
    vec2, Align, Color32, Context, Frame, Layout, Margin, PointerButton, Sense, TopBottomPanel,
    ViewportCommand,
};
use egui::{FontFamily, FontId, Image, RichText};
use serde::{Deserialize, Serialize};

use crate::{structs::WindowState, widgets::styled_button::StyledButton};

#[derive(Serialize, Deserialize, Default)]
pub struct TitleBar {
    pub window_state: WindowState,
    bar_height: f32,
}

impl TitleBar {
    pub fn new() -> Self {
        TitleBar {
            window_state: WindowState::default(),
            bar_height: 32.,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        let zoom_scale = ctx.pixels_per_point();
        let fixed_bar_height = self.bar_height / zoom_scale;
        let control_size = vec2(fixed_bar_height, fixed_bar_height);

        TopBottomPanel::top("title_bar")
            .frame(Frame::default().inner_margin(Margin::ZERO))
            .show(ctx, |ui| {
                self.handle_drag(ctx, ui);

                menu::bar(ui, |ui| {
                    ui.add_space(5. / zoom_scale);
                    ui.vertical(|ui| {
                        ui.add_space(3. / zoom_scale);
                        ui.label(RichText::new("msc").font(FontId::new(
                            32. / zoom_scale,
                            FontFamily::Name("logo".into()),
                        )));
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        self.window_control_buttons(ctx, ui, control_size);
                    });
                });
            });
    }

    fn handle_drag(&mut self, ctx: &Context, ui: &mut egui::Ui) {
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
    }

    fn window_control_buttons(&self, ctx: &Context, ui: &mut egui::Ui, size: egui::Vec2) {
        ui.spacing_mut().item_spacing = vec2(0., 0.);
        ui.add(
            StyledButton::new(
                size,
                &Image::new(include_image!("../../assets/icons/x.png")),
                || ctx.send_viewport_cmd(ViewportCommand::Close),
            )
            .with_hover_color(Color32::from_rgb(232, 17, 35)),
        );

        let min_max = if self.window_state.is_maximized {
            include_image!("../../assets/icons/restore.png")
        } else {
            include_image!("../../assets/icons/maximize.png")
        };

        ui.add(StyledButton::new(size, &Image::new(min_max), || {
            ctx.send_viewport_cmd(ViewportCommand::Maximized(!self.window_state.is_maximized))
        }));

        ui.add(StyledButton::new(
            size,
            &Image::new(include_image!("../../assets/icons/minimize.png")),
            || ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
        ));
    }
}
