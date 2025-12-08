use std::path::PathBuf;

use lofty::{error::LoftyError, file::{AudioFile, TaggedFileExt}, probe::Probe, tag::Accessor};

pub struct Metadata {
    title: String,
    artist: String,
    album: String,
    duration: f32,
}

impl Metadata {
    pub fn from_path(path: &PathBuf) -> Result<Self, LoftyError> {
        let file = Probe::open(path)?.read()?; 
        let props = file.properties();
        let tag = file.primary_tag().or_else(|| file.first_tag()).unwrap(); // CHANGE THIS UNWRAP

        let duration = props.duration().as_secs_f32();

        let title = tag.title().as_deref().unwrap_or("None").to_string();
        let artist = tag.artist().as_deref().unwrap_or("None").to_string();
        let album = tag.album().as_deref().unwrap_or("None").to_string();
        
        Ok(Metadata { title, artist, album, duration })
    }
}
