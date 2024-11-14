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
