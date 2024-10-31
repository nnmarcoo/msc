use eframe::egui::{self, Color32, Response, Ui, Vec2};
use std::ops::RangeInclusive;

/// Customizable slider widget.
///
/// ## Parameters:
/// - `value`: The current value of the slider (mutable).
/// - `range`: The inclusive range of the slider (e.g., 0.0..=100.0).
/// - `length`: Length of the slider track.
/// - `thickness`: Thickness of the slider track.
/// - `handle_size`: Radius of the handle (circle) when visible.
/// - `trail_color`: Color of the trailing portion of the slider.
pub fn color_slider_ui(
    ui: &mut Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    length: f32,
    thickness: f32,
    handle_size: f32,
    trail_color: Color32,
) -> Response {
    // 1. Decide size for the widget based on the length and thickness.
    let desired_size = Vec2::new(length, thickness);

    // 2. Allocate space for the slider.
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

    // 3. Handle interactions.
    if response.dragged() || response.clicked() {
        // Immediately set the value to the position of the pointer within the track.
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let new_value = egui::lerp(
                *range.start()..=*range.end(),
                (pointer_pos.x - rect.left()) / rect.width(),
            );
            *value = new_value.clamp(*range.start(), *range.end());
            response.mark_changed(); // Notify that the value has changed.
        }
    }

    // 4. Paint the slider.
    if ui.is_rect_visible(rect) {
        // Set the background color and handle color.
        let background_color = Color32::from_rgb(60, 60, 60); // 3c3c3c for both background and handle
        let value_position = egui::lerp(
            rect.left()..=rect.right(),
            (*value - *range.start()) / (*range.end() - *range.start()),
        );

        // Paint the track background.
        ui.painter()
            .rect_filled(rect, 0.5 * thickness, background_color);

        // Paint the trailing portion of the slider if value > minimum.
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

        // Draw the handle if hovered or dragged.
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

/// Usage: Allows for idiomatic use with `ui.add(custom_slider(...))`.
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
