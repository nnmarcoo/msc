use eframe::egui::Color32;

pub fn format_seconds(seconds: f32) -> String {
    let total_seconds = seconds.trunc() as u64;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

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
