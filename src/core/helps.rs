use std::{
    collections::HashMap,
    fs::read_dir,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use blake3::Hash;
use egui::{
    epaint::text::{FontInsert, InsertFontFamily},
    ColorImage, Context, FontData, FontFamily, TextureHandle, TextureOptions,
};
use image::{imageops::FilterType, DynamicImage};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::{structs::State, track::Track};

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

pub fn collect_audio_files(dir: &Path) -> HashMap<Hash, Track> {
    let mut map = HashMap::new();

    if let Ok(entries) = read_dir(dir) {
        let entries: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();

        // Collect results in parallel into a Vec, then merge
        let results: Vec<(Hash, Track)> = entries
            .par_iter()
            .flat_map(|path| {
                let mut local_results = Vec::new();
                if path.is_file() {
                    if let Some(extension) = path.extension() {
                        let ext = extension.to_string_lossy().to_lowercase();
                        if ["mp3", "flac", "m4a", "wav", "ogg"].contains(&ext.as_str()) {
                            if let Some(path_str) = path.to_str() {
                                if let Some(track) = Track::new(path_str) {
                                    if let Ok(bytes) = std::fs::read(path) {
                                        let hash = blake3::hash(&bytes);
                                        local_results.push((hash, track));
                                    }
                                }
                            }
                        }
                    }
                } else if path.is_dir() {
                    let sub_map = collect_audio_files(path);
                    for (hash, track) in sub_map {
                        local_results.push((hash, track));
                    }
                }
                local_results
            })
            .collect();

        for (hash, track) in results {
            map.insert(hash, track);
        }
    }
    map
}

pub fn init(state: &mut State) {
    if state.is_initialized {
        return;
    }
    state.is_initialized = true;

    state.library = collect_audio_files(Path::new(&state.audio_directory));
}
