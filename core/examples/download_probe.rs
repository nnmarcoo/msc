//! Live download check. Run with:
//! `cargo run -p verse-core --features explore --example download_probe`
//!
//! Searches for one recording, downloads it, tags it, and reads it back through
//! [`verse_core::Track::from_path`] — the same reader the library scans with.
//! Needs `yt-dlp` on PATH; without it the run reports that and stops, since a
//! missing tool is not a failing library.

use std::path::PathBuf;

use verse_core::Track;
use verse_core::explore::{
    Destination, DownloadSource, Innertube, MusicSource, YtDlp, path_for, write_tags,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let downloader = YtDlp::new();

    let Some(version) = downloader.version().await else {
        println!("yt-dlp is not on PATH — run scripts/setup-explore.ps1");
        return;
    };
    println!("yt-dlp {version}");

    let source = Innertube::new();
    let results = match source.search("radiohead reckoner", 1).await {
        Ok(results) => results,
        Err(e) => {
            println!("SEARCH FAILED: {e}");
            return;
        }
    };

    let Some(found) = results.into_iter().next() else {
        println!("search returned nothing");
        return;
    };

    println!(
        "found: {} — {:?} ({:?}s)",
        found.title, found.artist, found.duration
    );

    let scratch: PathBuf = std::env::temp_dir().join("verse-download-probe");
    let staging = scratch.join("staging");

    let mut last = -1_i32;
    let audio = match downloader
        .fetch(&found.id, &staging, |progress| {
            if let Some(fraction) = progress.fraction {
                let step = (fraction * 10.0) as i32;
                if step > last {
                    last = step;
                    println!("  {:>3.0}%", fraction * 100.0);
                }
            }
        })
        .await
    {
        Ok(path) => path,
        Err(e) => {
            println!("DOWNLOAD FAILED: {e}");
            return;
        }
    };

    println!("downloaded: {}", audio.display());

    let cover = match &found.cover_url {
        Some(url) => fetch_cover(url).await,
        None => None,
    };
    println!("cover: {} bytes", cover.as_ref().map_or(0, Vec::len));

    let into = Destination {
        album_artist: found.artist.clone(),
        album: found.album.clone(),
        year: None,
        track_number: None,
        disc_number: None,
        cover,
    };

    if let Err(e) = write_tags(&audio, &found, &into) {
        println!("TAGGING FAILED: {e}");
        return;
    }

    let filed = path_for(&scratch, &found, &into);
    if let Some(parent) = filed.parent() {
        std::fs::create_dir_all(parent).expect("the album directory");
    }
    std::fs::rename(&audio, &filed).expect("filing the track");

    println!("filed: {}", filed.display());

    match Track::from_path(&filed) {
        Ok(track) => {
            println!("READ BACK BY THE LIBRARY:");
            println!("  title:        {:?}", track.title());
            println!("  artist:       {:?}", track.track_artist());
            println!("  album:        {:?}", track.album());
            println!("  album artist: {:?}", track.album_artist());
            println!("  duration:     {:.1}s", track.duration());
            println!(
                "  artwork:      {} bytes",
                verse_core::extract_artwork_bytes(&filed).map_or(0, |a| a.len())
            );
        }
        Err(e) => println!("THE LIBRARY COULD NOT READ IT: {e}"),
    }

    println!("\nleaving {} for inspection", scratch.display());
}

async fn fetch_cover(url: &str) -> Option<Vec<u8>> {
    let response = reqwest::get(url).await.ok()?;
    response
        .bytes()
        .await
        .ok()
        .map(|bytes| bytes.to_vec())
        .filter(|bytes| !bytes.is_empty())
}
