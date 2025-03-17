use egui::ColorImage;
use image::{imageops::FilterType, DynamicImage};

pub fn format_seconds(seconds: f32) -> String {
    let minutes = (seconds / 60.) as u32;
    let seconds = (seconds % 60.) as u32;

    format!("{:02}:{:02}", minutes, seconds)
}

pub fn load_image(image: DynamicImage, width: u32, height: u32) -> ColorImage {
    let rgba_image = image
        .clone()
        .resize_exact(width, height, FilterType::Lanczos3)
        .to_rgba8();
    let dimensions = rgba_image.dimensions();
    let pixels = rgba_image.into_raw();

    ColorImage::from_rgba_unmultiplied([dimensions.0 as usize, dimensions.1 as usize], &pixels)
}
