use eframe::egui::{
    include_image,
    menu::{self},
    vec2, Align, Color32, Context, Frame, ImageButton, Layout, Margin, PointerButton, Sense,
    TextEdit, TopBottomPanel, ViewportCommand,
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
                                [48., 48.],
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
                                [48., 48.],
                                ImageButton::new(include_image!("../../assets/icons/restore.png")),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(ViewportCommand::Maximized(false));
                        }
                    } else {
                        if ui
                            .add_sized(
                                [48., 48.],
                                ImageButton::new(include_image!("../../assets/icons/maximize.png")),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(ViewportCommand::Maximized(true));
                        }
                    }

                    if ui
                        .add_sized(
                            [48., 48.],
                            ImageButton::new(include_image!("../../assets/icons/minimize.png")),
                        )
                        .clicked()
                    {
                        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                    }

                    ui.add(
                        TextEdit::singleline(&mut app.song_search)
                            .hint_text("🔍 Search a song")
                            .desired_width(150.),
                    );

                    ui.add_space(ui.available_width() - 47.);

                    ui.allocate_ui(vec2(28., 28.), |ui| {
                        ui.menu_image_button(
                            include_image!("../../assets/icons/settings.png"),
                            |ui| {
                                if ui.button("About").clicked() {}
                                if ui.button("Help").clicked() {}
                                if ui.button("Update").clicked() {}
                                ui.separator();
                                if ui.button("Settings").clicked() {}
                            },
                        );
                    });
                });
            });
        });
}
