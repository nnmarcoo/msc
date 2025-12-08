use dashmap::DashMap;
use blake3::{Hash, Hasher};
use image::DynamicImage;
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{path::PathBuf, sync::Arc};
use std::path::Path;

pub struct ArtCache {
    cache: DashMap<Hash, Arc<DynamicImage>>,
}

impl ArtCache {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }
    
    fn artwork_id(album: &str, artist: &str) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(album.as_bytes());
        hasher.update(artist.as_bytes());
        hasher.finalize()
    }
    
    pub fn get_or_load(&self, track_path: &Path, album: &str, artist: &str) 
        -> Option<Arc<DynamicImage>> 
    {
        let id = Self::artwork_id(album, artist);
        
        if let Some(img) = self.cache.get(&id) {
            return Some(img.value().clone());
        }
        
        if let Some(image_data) = Self::extract_image(track_path) {
            let arc_data = Arc::new(image_data);
            self.cache.insert(id, arc_data.clone());
            Some(arc_data)
        } else {
            None
        }
    }
    
    fn extract_image(path: &Path) -> Option<DynamicImage> {
        let file = Probe::open(path).ok()?.read().ok()?;
        let tag = file.primary_tag().or_else(|| file.first_tag())?;

        let picture = tag.pictures().first()?;
        let image = image::load_from_memory(picture.data()).ok()?;

        Some(image)
    }

    pub fn precache(&self, tracks: &[(PathBuf, String, String)]) {
        tracks.par_iter().for_each(|(path, album, artist)| {
            self.get_or_load(path, album, artist);
        });
    }
    
    pub fn clear(&self) {
        self.cache.clear();
    }
    
    pub fn len(&self) -> usize {
        self.cache.len()
    }
}