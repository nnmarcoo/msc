use std::{
    fs::read_dir,
    path::Path,
    sync::{Arc, Mutex},
};

use blake3::Hash;
use dashmap::DashMap;
use egui::{
    epaint::text::{FontInsert, InsertFontFamily},
    ColorImage, Context, FontData, FontFamily, TextureHandle, TextureOptions,
};
use image::{imageops::FilterType, DynamicImage};
use rayon::iter::{ParallelBridge, ParallelIterator};

use super::track::Track;

pub fn amp_to_db(v: f32) -> f32 {
    35. * v.log10()
}

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

pub fn collect_audio_files(dir: &Path) -> DashMap<Hash, Track> {
    let map = DashMap::new();

    if let Ok(entries) = read_dir(dir) {
        entries.par_bridge().for_each(|res| {
            if let Ok(entry) = res {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext = ext.to_lowercase();
                        if ["mp3", "flac", "wav", "ogg"].contains(&ext.as_str()) {
                            if let Some(path_str) = path.to_str() {
                                if let Some(track) = Track::new(path_str) {
                                    map.insert(track.hash, track);
                                }
                            }
                        }
                    }
                } else if path.is_dir() {
                    let sub_map = collect_audio_files(&path);
                    for (hash, track) in sub_map {
                        map.insert(hash, track);
                    }
                }
            }
        });
    }
    map
}
