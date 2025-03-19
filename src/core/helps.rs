use std::sync::{Arc, Mutex};

use egui::{
    epaint::text::{FontInsert, InsertFontFamily},
    ColorImage, Context, FontData, FontFamily, TextureHandle, TextureOptions,
};
use image::{imageops::FilterType, DynamicImage};

pub fn format_seconds(seconds: f32) -> String {
    let minutes = (seconds / 60.) as u32;
    let seconds = (seconds % 60.) as u32;

    format!("{:02}:{:02}", minutes, seconds)
}

pub fn add_font(ctx: &Context) {
    ctx.add_font(FontInsert::new(
        "Not Sure if Weird or Just Regular",
        FontData::from_static(include_bytes!(
            "../../assets/Not Sure if Weird or Just Regular.ttf"
        )),
        vec![InsertFontFamily {
            family: FontFamily::Name("logo".into()),
            priority: egui::epaint::text::FontPriority::Highest,
        }],
    ));
}

#[allow(dead_code)]
pub fn load_image(
    image_path: String,
    width: u32,
    height: u32,
    ctx: &Context,
    texture_arc: Arc<Mutex<Option<TextureHandle>>>,
) {
    let image_path_clone = image_path.clone();
    let ctx = ctx.clone();

    rayon::spawn(move || {
        let img = image::open(&image_path_clone).unwrap_or_else(|err| {
            eprintln!(
                "Failed to open image at {}: {}. Creating an empty fallback image.",
                image_path_clone, err
            );
            DynamicImage::new_rgba8(width, height)
        });

        let resized = img.resize_exact(width, height, FilterType::Lanczos3);
        let rgba = resized.to_rgba8();
        let (w, h) = rgba.dimensions();

        let color_image =
            ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba.into_raw());

        let texture = ctx.load_texture(&image_path_clone, color_image, TextureOptions::NEAREST);

        *texture_arc.lock().unwrap() = Some(texture);
    });
}
