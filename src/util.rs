use eframe::egui::{Color32, ColorImage};
use image::{imageops::FilterType, load_from_memory};

use lofty::{
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::ItemKey,
};

pub struct AudioMetadata {
    pub title: String,
    pub artist: String,
    pub image: ColorImage, // TODO: add place holder
    pub duration: f32,
}

pub fn seconds_to_string(seconds: f32) -> String {
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

// change and make separate struct for tracks
pub fn get_audio_metadata(file_path: &str) -> Result<AudioMetadata, Box<dyn std::error::Error>> {
    let tagged_file = Probe::open(file_path)?.read()?;
    let properties = tagged_file.properties();
    let tag = tagged_file.primary_tag();

    let title = tag
        .and_then(|t| t.get_string(&ItemKey::TrackTitle).map(String::from))
        .unwrap_or("NA".to_string());
    let artist = tag
        .and_then(|t| t.get_string(&ItemKey::AlbumArtist).map(String::from))
        .unwrap_or("NA".to_string());

    let duration = properties.duration().as_secs_f32();

    let mut image = ColorImage::example(); // change

    if let Some(picture) = tagged_file
        .primary_tag()
        .and_then(|tag| tag.pictures().first())
    {
        let image_data = picture.data();
        let img =
            load_from_memory(image_data)
                .ok()
                .unwrap()
                .resize_exact(50, 50, FilterType::Lanczos3);
        let img = img.resize_exact(50, 50, FilterType::Lanczos3);

        let rgba_img = img.to_rgba8();
        let size = [rgba_img.width() as usize, rgba_img.height() as usize];
        let pixels = rgba_img.into_raw();

        image = ColorImage::from_rgba_unmultiplied(size, &pixels)
    }

    Ok(AudioMetadata {
        title,
        artist,
        image,
        duration,
    })
}
