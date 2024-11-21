use std::{fs::read_dir, io::Cursor, path::Path};

use eframe::egui::ColorImage;

use image::{imageops::FilterType, load_from_memory};
use lofty::{
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::ItemKey,
};

pub struct Track {
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub image: ColorImage,
    pub duration: f32,
}

impl Track {
    pub fn default() -> Self {
        Track {
            file_path: String::from("-"),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            image: ColorImage::example(),
            duration: 0.,
        }
    }

    pub fn new(path: &str) -> Self {
        let tagged_file = Probe::open(path).unwrap().read().unwrap();
        let properties = tagged_file.properties();
        let tag = tagged_file.primary_tag();

        let title = tag
            .and_then(|t| t.get_string(&ItemKey::TrackTitle).map(String::from))
            .unwrap_or(
                Path::new(path)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            );

        let artist = tag
            .and_then(|t| t.get_string(&ItemKey::AlbumArtist).map(String::from))
            .unwrap_or(String::new());

        let album = tag
            .and_then(|t| t.get_string(&ItemKey::AlbumTitle).map(String::from))
            .unwrap_or(String::new());

        let duration = properties.duration().as_secs_f32();

        let image = if let Some(picture) = tagged_file
            .primary_tag()
            .and_then(|tag| tag.pictures().first())
        {
            let image_data = picture.data();

            let img =
                load_from_memory(image_data)
                    .ok()
                    .unwrap()
                    .resize(48, 48, FilterType::Lanczos3);

            let rgba_img = img.to_rgba8();
            let size = [rgba_img.width() as usize, rgba_img.height() as usize];
            let pixels = rgba_img.into_raw();

            ColorImage::from_rgba_unmultiplied(size, &pixels)
        } else {
            let img = image::load(
                Cursor::new(include_bytes!("../../assets/icons/defaultborder.png")),
                image::ImageFormat::Png,
            )
            .unwrap();

            let img = img.to_rgba8();
            let (width, height) = img.dimensions();

            ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &img)
        };

        Track {
            file_path: path.to_string(),
            title,
            artist,
            album,
            image,
            duration,
        }
    }

    fn count_audio_files(path: &str) -> Result<usize, std::io::Error> {
        // do I need this
        let extensions = ["mp3", "flac", "m4a", "ogg"];
        let mut count = 0;

        for entry in read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.is_file() {
                if let Some(extension) = file_path.extension() {
                    if let Some(ext) = extension.to_str() {
                        if extensions.contains(&ext.to_lowercase().as_str()) {
                            count += 1;
                        }
                    }
                }
            }
        }

        Ok(count)
    }
}
