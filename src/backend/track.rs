use eframe::egui::ColorImage;

use lofty::{
  file::{AudioFile, TaggedFileExt},
  probe::Probe,
  tag::ItemKey,
};

pub struct Track {
    pub title: String,
    pub artist: String,
    pub image: ColorImage, // TODO: add place holder
    pub duration: f32,
}

impl Track {
  pub fn new(file_path: &str) -> Self {
    let tagged_file = Probe::open(file_path).unwrap().read().unwrap();
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

    Track {
      title,
      artist,
      image,
      duration,
    }
  }
}
