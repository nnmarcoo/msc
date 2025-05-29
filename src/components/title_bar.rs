use eframe::egui::{
    include_image,
    menu::{self},
    vec2, Align, Color32, Context, Frame, Layout, Margin, PointerButton, Sense, TopBottomPanel,
    ViewportCommand,
};
use egui::{FontFamily, FontId, Image, RichText, Ui, Vec2};

use crate::{
    structs::{State, View},
    widgets::styled_button::StyledButton,
};

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct TitleBar {
    size: Vec2,
}

impl TitleBar {
    pub fn new() -> Self {
        TitleBar {
            size: vec2(32., 32.),
        }
    }

    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        TopBottomPanel::top("title_bar")
            .frame(Frame::default().inner_margin(Margin::ZERO))
            .show(ctx, |ui| {
                self.handle_drag(ctx, ui, state);

                menu::bar(ui, |ui| {
                    ui.add_space(5.);
                    ui.vertical(|ui| {
                        ui.add_space(3.);
                        ui.label(
                            RichText::new("msc")
                                .font(FontId::new(32., FontFamily::Name("logo".into()))),
                        );
                    });

                    ui.add(
                        StyledButton::new(
                            self.size,
                            &Image::new(include_image!("../../assets/icons/playlists.png")),
                            || state.view = View::Playlists,
                        )
                        .with_hover_text("Playlists"),
                    );

                    ui.add(
                        StyledButton::new(
                            self.size,
                            &Image::new(include_image!("../../assets/icons/library.png")),
                            || state.view = View::Library,
                        )
                        .with_hover_text("Library"),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        self.window_control_buttons(ctx, ui, self.size, state);
                    });
                });
            });
    }

    fn handle_drag(&mut self, ctx: &Context, ui: &mut egui::Ui, state: &mut State) {
        state.is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        let res = ui.interact(ui.max_rect(), ui.id(), Sense::click_and_drag());

        if res.drag_started_by(PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            state.is_dragging = true;
        }
        if res.drag_stopped() {
            state.is_dragging = false;
        }
        if res.double_clicked_by(PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::Maximized(!state.is_maximized));
        }
    }

    fn window_control_buttons(&self, ctx: &Context, ui: &mut Ui, size: Vec2, state: &mut State) {
        ui.spacing_mut().item_spacing = vec2(0., 0.);
        ui.add(
            StyledButton::new(
                size,
                &Image::new(include_image!("../../assets/icons/x.png")),
                || ctx.send_viewport_cmd(ViewportCommand::Close),
            )
            .with_hover_color(Color32::from_rgb(232, 17, 35))
            .with_hover_text("Close"),
        );

        let (min_max, min_max_text) = if state.is_maximized {
            (include_image!("../../assets/icons/restore.png"), "Restore")
        } else {
            (
                include_image!("../../assets/icons/maximize.png"),
                "Maximize",
            )
        };

        ui.add(
            StyledButton::new(size, &Image::new(min_max), || {
                ctx.send_viewport_cmd(ViewportCommand::Maximized(!state.is_maximized))
            })
            .with_hover_text(min_max_text),
        );

        ui.add(
            StyledButton::new(
                size,
                &Image::new(include_image!("../../assets/icons/minimize.png")),
                || ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
            )
            .with_hover_text("Minimize"),
        );
    }
}
