use lofty::{file::TaggedFileExt, picture::PictureType, probe::Probe};
use std::path::Path;

pub fn extract_artwork_bytes(path: &Path) -> Option<Vec<u8>> {
    let file = Probe::open(path).ok()?.read().ok()?;
    let tag = file.primary_tag().or_else(|| file.first_tag())?;
    let pictures = tag.pictures();

    pictures
        .iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| pictures.first())
        .map(|p| p.data().to_vec())
}
