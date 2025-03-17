use eframe::egui::{self, Color32, Response, Ui, Vec2};
use std::ops::RangeInclusive;

pub fn color_slider_ui(
    ui: &mut Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    length: f32,
    thickness: f32,
    handle_size: f32,
    trail_color: Color32,
) -> Response {
    let desired_size = Vec2::new(length, thickness);

    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

    if response.dragged() || response.is_pointer_button_down_on() {
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let new_value = egui::lerp(
                *range.start()..=*range.end(),
                (pointer_pos.x - rect.left()) / rect.width(),
            );
            *value = new_value.clamp(*range.start(), *range.end());
            response.mark_changed();
        }
    }

    if ui.is_rect_visible(rect) {
        let background_color = Color32::from_rgb(60, 60, 60);
        let value_position = egui::lerp(
            rect.left()..=rect.right(),
            (*value - *range.start()) / (*range.end() - *range.start()),
        );

        ui.painter()
            .rect_filled(rect, 0.5 * thickness, background_color);

        if *value > *range.start() + 0.05 {
            ui.painter().rect_filled(
                egui::Rect::from_min_max(
                    rect.left_top(),
                    egui::pos2(value_position, rect.bottom()),
                ),
                0.5 * thickness,
                trail_color,
            );
        }

        if response.hovered() || response.dragged() {
            let handle_center = egui::pos2(value_position, rect.center().y);
            ui.painter().circle(
                handle_center,
                handle_size,
                background_color,
                ui.style().interact(&response).fg_stroke,
            );
        }
    }

    response
}

pub fn color_slider(
    value: &mut f32,
    range: RangeInclusive<f32>,
    length: f32,
    thickness: f32,
    handle_size: f32,
    trail_color: Color32,
) -> impl egui::Widget + '_ {
    move |ui: &mut Ui| {
        color_slider_ui(
            ui,
            value,
            range,
            length,
            thickness,
            handle_size,
            trail_color,
        )
    }
}
