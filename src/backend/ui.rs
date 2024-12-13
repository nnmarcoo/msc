use std::collections::HashSet;

use eframe::egui::{Color32, Response};

pub fn format_seconds(seconds: f32) -> String {
    let minutes = (seconds / 60.) as u32;
    let seconds = (seconds % 60.) as u32;

    format!("{:02}:{:02}", minutes, seconds)
}

pub fn get_volume_color(value: f32) -> Color32 {
    let low_blue = Color32::from_rgb(0, 50, 80);
    let blue = Color32::from_rgb(0, 92, 128);
    let high_blue = Color32::from_rgb(0, 200, 255);

    // Clamp the value between 0 and 2
    let clamped_value = value.clamp(0.0, 2.0);

    if clamped_value <= 1.0 {
        // Interpolate between green and blue
        let t = clamped_value;
        let r = (low_blue.r() as f32 * (1.0 - t) + blue.r() as f32 * t) as u8;
        let g = (low_blue.g() as f32 * (1.0 - t) + blue.g() as f32 * t) as u8;
        let b = (low_blue.b() as f32 * (1.0 - t) + blue.b() as f32 * t) as u8;
        Color32::from_rgb(r, g, b)
    } else {
        // Interpolate between blue and red
        let t = clamped_value - 1.0;
        let r = (blue.r() as f32 * (1.0 - t) + high_blue.r() as f32 * t) as u8;
        let g = (blue.g() as f32 * (1.0 - t) + high_blue.g() as f32 * t) as u8;
        let b = (blue.b() as f32 * (1.0 - t) + high_blue.b() as f32 * t) as u8;
        Color32::from_rgb(r, g, b)
    }
}

pub fn toggle_row_selection(selection: &mut HashSet<usize>, index: usize, row_response: &Response) {
    if row_response.clicked() {
        if selection.contains(&index) {
            selection.remove(&index);
        } else {
            selection.insert(index);
        }
    }
}
