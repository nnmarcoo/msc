use eframe::egui::{
    include_image, menu, Align, Color32, Context, Frame, ImageButton, Layout, Margin,
    PointerButton, Sense, TopBottomPanel, ViewportCommand,
};

use crate::msc::Msc;

pub fn show_title_bar(app: &mut Msc, ctx: &Context) {
    TopBottomPanel::top("title_bar")
        .frame(Frame::default().inner_margin(Margin::ZERO))
        .show(ctx, |ui| {
            let res = ui.interact(ui.max_rect(), ui.id(), Sense::click_and_drag());

            if res.drag_started_by(PointerButton::Primary) {
                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                app.is_dragging = true;
            }

            if res.drag_stopped() {
                app.is_dragging = false;
            }

            if res.double_clicked_by(PointerButton::Primary) {
                ctx.send_viewport_cmd(ViewportCommand::Maximized(!app.is_maximized));
            }

            menu::bar(ui, |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.scope(|ui| {
                        ui.style_mut().visuals.widgets.hovered.weak_bg_fill =
                            Color32::from_rgb(232, 17, 35);
                        if ui
                            .add_sized(
                                [45., 50.],
                                ImageButton::new(include_image!("../../assets/icons/x.png")),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(ViewportCommand::Close);
                        }
                    });

                    if app.is_maximized {
                        if ui
                            .add_sized(
                                [45., 50.],
                                ImageButton::new(include_image!("../../assets/icons/restore.png")),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(ViewportCommand::Maximized(false));
                        }
                    } else {
                        if ui
                            .add_sized(
                                [45., 50.],
                                ImageButton::new(include_image!("../../assets/icons/maximize.png")),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(ViewportCommand::Maximized(true));
                        }
                    }

                    if ui
                        .add_sized(
                            [45., 50.],
                            ImageButton::new(include_image!("../../assets/icons/minimize.png")),
                        )
                        .clicked()
                    {
                        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                    }
                });
            });
        });
}
