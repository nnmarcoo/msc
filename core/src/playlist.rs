#[derive(Debug, Clone)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub cover_track_id: Option<i64>,
    pub track_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
