//! Live check that a listing's id is repaired when it names the wrong
//! recording. Run with:
//! `cargo run -p verse-core --features explore --example resolve_probe`

use verse_core::explore::{DownloadSource, Innertube, MusicSource, YtDlp, for_download};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let source = Innertube::new();
    let fetcher = YtDlp::new();

    if fetcher.version().await.is_none() {
        println!("yt-dlp is not installed — run scripts/setup-explore.ps1");
        return;
    }

    let album_id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "MPREb_juJ08V7MojN".to_owned());

    let album = match source.album(&album_id).await {
        Ok(album) => album,
        Err(e) => {
            println!("ALBUM FAILED: {e}");
            return;
        }
    };

    println!("{} — {:?}\n", album.title, album.artist);

    for track in &album.tracks {
        let stated = track.duration.unwrap_or(0);
        let actual = fetcher.duration_of(&track.id).await;

        let resolved = for_download(&source, &fetcher, track).await;

        let verdict = match (&resolved.substituted, actual) {
            (Some(_), Some(was)) => {
                let now = fetcher.duration_of(&resolved.id).await;
                format!("REPAIRED {was}s -> {:?}s via {}", now, resolved.id)
            }
            (None, Some(was)) if was.abs_diff(stated) > 20 => {
                "MISMATCHED, no better candidate found".to_owned()
            }
            (None, _) => "ok".to_owned(),
            (Some(_), None) => "substituted".to_owned(),
        };

        println!(
            "  {:<48} stated {stated:>4}s  actual {:>5}  {verdict}",
            track.title,
            actual.map_or_else(|| "?".to_owned(), |d| format!("{d}s"))
        );
    }
}
