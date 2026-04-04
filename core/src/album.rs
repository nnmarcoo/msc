#[derive(Debug, Clone)]
pub struct Album {
    pub id: i64,
    pub name: String,
    pub artist: Option<String>,
    pub year: Option<u32>,
    pub sample_track_path: Option<String>,
}
