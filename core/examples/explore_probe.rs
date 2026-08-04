//! Live check against YouTube Music. Run with:
//! `cargo run -p verse-core --features explore --example explore_probe`
//!
//! The unit tests read fixtures, which prove the parser reads the shape it was
//! written against. This proves the shape is still what YouTube sends, which is
//! the half a fixture can never tell you.

use verse_core::explore::{Innertube, MusicSource};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let source = Innertube::new();

    match source.new_albums(14).await {
        Ok(albums) => {
            println!("NEW RELEASES: {} albums", albums.len());
            for a in &albums {
                println!(
                    "  [{:6}] {} | {:?} | art={}",
                    a.release.label(),
                    a.title,
                    a.artist,
                    a.cover_url.is_some()
                );
            }
        }
        Err(e) => println!("NEW RELEASES FAILED: {e}"),
    }

    match source.search_albums("radiohead", 5).await {
        Ok(albums) => {
            println!("ALBUM SEARCH: {} results", albums.len());
            for a in &albums {
                println!(
                    "  {} | {} | {:?} | {:?} | art={}",
                    a.id,
                    a.title,
                    a.artist,
                    a.year,
                    a.cover_url.is_some()
                );
            }
        }
        Err(e) => println!("ALBUM SEARCH FAILED: {e}"),
    }

    match source.search("radiohead in rainbows", 5).await {
        Ok(found) => {
            println!("SEARCH: {} results", found.len());
            for song in &found {
                println!(
                    "  {} | {} | {:?} | {:?} | {:?}s | album_id={:?}",
                    song.id, song.title, song.artist, song.album, song.duration, song.album_id
                );
            }

            if let Some(id) = found.first().and_then(|s| s.album_id.clone()) {
                match source.album(&id).await {
                    Ok(album) => {
                        println!(
                            "\nALBUM: {} by {:?} ({:?}) — {} tracks",
                            album.title,
                            album.artist,
                            album.year,
                            album.tracks.len()
                        );
                        println!("  cover: {:?}", album.cover_url);
                        for track in album.tracks.iter().take(3) {
                            println!("    {} | {} | {:?}s", track.id, track.title, track.duration);
                        }
                    }
                    Err(e) => println!("ALBUM FAILED: {e}"),
                }
            }

            if let Some(seed) = found.first().map(|s| s.id.clone()) {
                match source.similar(&seed, 5).await {
                    Ok(similar) => {
                        println!("\nSIMILAR to {seed}: {} tracks", similar.len());
                        for song in &similar {
                            println!("  {} | {} | {:?}", song.id, song.title, song.artist);
                        }
                        assert!(
                            !similar.iter().any(|s| s.id == seed),
                            "the seed must not be returned as similar to itself"
                        );
                    }
                    Err(e) => println!("SIMILAR FAILED: {e}"),
                }
            }
        }
        Err(e) => println!("SEARCH FAILED: {e}"),
    }
}
