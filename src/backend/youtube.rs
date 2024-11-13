use rusty_ytdl::search::{SearchResult, YouTube};

pub async fn search(query: String) -> Vec<SearchResult> {
    let youtube = YouTube::new().unwrap();
    let res = youtube.search(query, None);
    res.await.unwrap()
}
