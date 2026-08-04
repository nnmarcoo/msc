//! Async side effects: the folder picker, the library scan, and artwork decoding.
//!
//! Each is a `Task` that ends in one [`Message`], so `update` stays the only
//! place state changes. The work itself lives elsewhere — scanning in
//! [`verse_core::Library`], decoding in [`crate::artwork::decode`] — and what is
//! here is only the moving of it off the frame thread and the naming of what
//! comes back. Anything that can fail answers with a message rather than
//! unwinding, since a task that dies silently leaves a pane waiting forever.
//!
//! The explore tasks carry a [`crate::explore::Generation`] out and back
//! unchanged. They do not decide whether their own reply is still wanted —
//! `update` does, against state a task cannot see — so the generation is
//! payload here and a verdict only once it lands.
//!
//! `search` sleeps before it asks. That is the debounce, and it lives in the
//! task rather than in a timer on the app because a `Task` is already the thing
//! being spawned per keystroke: sleeping inside it means the superseded ones
//! wake to find themselves stale and their replies dropped, with no timer state
//! to keep in step with the query.

use std::path::PathBuf;

use iced::Task;
use verse_core::Library;

use crate::app::Message;
use crate::artwork::{Decoded, Job, Source, decode};

#[cfg(feature = "explore")]
use std::sync::Arc;
#[cfg(feature = "explore")]
use verse_core::explore::{Destination, DownloadSource, Found, Innertube, MusicSource};

#[cfg(feature = "explore")]
use iced::futures::SinkExt;

#[cfg(feature = "explore")]
use crate::explore::{
    ALBUM_LIMIT, BROWSE_LIMIT, Generation, Results, SEARCH_LIMIT, SIMILAR_LIMIT, Stage,
};

#[cfg(feature = "explore")]
const DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(300);

pub fn pick_library_folder() -> Task<Message> {
    Task::future(async {
        match rfd::AsyncFileDialog::new()
            .set_title("Select Music Library Folder")
            .pick_folder()
            .await
        {
            Some(handle) => Message::FolderPicked(handle.path().to_path_buf()),
            None => Message::Noop,
        }
    })
}

pub fn decode_art(job: Job, master: Option<Source>) -> Task<Message> {
    Task::future(async move {
        let (track, bucket) = (job.track, job.bucket);

        let decoded = tokio::task::spawn_blocking(move || decode(&job, master))
            .await
            .unwrap_or_else(|_| Decoded::nothing(track, bucket));

        Message::ArtDecoded(Box::new(decoded))
    })
}

pub fn scan(root: PathBuf) -> Task<Message> {
    Task::future(async move {
        let result =
            tokio::task::spawn_blocking(move || -> Result<(), verse_core::LibraryError> {
                let mut library = Library::open()?;
                library.scan(&root)?;
                Ok(())
            })
            .await;

        match result {
            Ok(Ok(())) => Message::ScanFinished(Ok(())),
            Ok(Err(e)) => Message::ScanFinished(Err(e.to_string())),
            Err(_) => Message::ScanFinished(Err("scan thread panicked".into())),
        }
    })
}

#[cfg(feature = "explore")]
pub fn search(source: Arc<Innertube>, generation: Generation, query: String) -> Task<Message> {
    Task::future(async move {
        tokio::time::sleep(DEBOUNCE).await;

        let (albums, tracks) = tokio::join!(
            source.search_albums(&query, ALBUM_LIMIT),
            source.search(&query, SEARCH_LIMIT)
        );

        let stage = match tracks {
            Ok(tracks) => Stage::Results(Box::new(Results {
                albums: albums.unwrap_or_default(),
                tracks,
            })),
            Err(e) => Stage::Failed(e.to_string()),
        };

        Message::ExploreSettled(generation, Box::new(stage))
    })
}

#[cfg(feature = "explore")]
pub fn similar(source: Arc<Innertube>, generation: Generation, id: String) -> Task<Message> {
    Task::future(async move {
        let stage = match source.similar(&id, SIMILAR_LIMIT).await {
            Ok(found) => Stage::Similar(id, found),
            Err(e) => Stage::Failed(e.to_string()),
        };

        Message::ExploreSettled(generation, Box::new(stage))
    })
}

/// Builds the landing feed's shelves.
///
/// The feed is one Innertube call, split into shelves here rather than fetched
/// per shelf: the endpoint answers a mixed pool of releases and already carries
/// the kind of each, so a second request would ask the same question twice. A
/// shelf that ends up empty is dropped by the pane rather than drawn as a
/// heading over nothing.
#[cfg(feature = "explore")]
pub fn browse(source: Arc<Innertube>, generation: Generation) -> Task<Message> {
    Task::future(async move {
        let stage = match source.new_albums(BROWSE_LIMIT).await {
            Ok(albums) => Stage::Browse(shelves(albums)),
            Err(e) => Stage::Failed(e.to_string()),
        };

        Message::ExploreSettled(generation, Box::new(stage))
    })
}

#[cfg(feature = "explore")]
fn shelves(albums: Vec<verse_core::explore::FoundAlbum>) -> Vec<crate::explore::Shelf> {
    use verse_core::explore::Release;

    let mut records = Vec::new();
    let mut extended = Vec::new();

    for album in albums {
        match album.release {
            Release::Ep => extended.push(album),
            _ => records.push(album),
        }
    }

    [("New albums", records), ("New EPs", extended)]
        .into_iter()
        .filter(|(_, albums)| !albums.is_empty())
        .map(|(label, albums)| crate::explore::Shelf {
            label: label.to_owned(),
            albums,
        })
        .collect()
}

#[cfg(feature = "explore")]
pub fn check_fetcher() -> Task<Message> {
    Task::future(async {
        let fetcher = verse_core::explore::YtDlp::new();
        Message::FetcherChecked(fetcher.available().await)
    })
}

#[cfg(feature = "explore")]
pub fn download(found: Found, into: Destination, root: PathBuf) -> Task<Message> {
    let id = found.id.clone();

    Task::stream(iced::stream::channel(
        16,
        move |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
            let (progress, mut updates) = tokio::sync::mpsc::channel::<f32>(16);
            let reporting = id.clone();

            let mut relay = sender.clone();
            tokio::spawn(async move {
                while let Some(fraction) = updates.recv().await {
                    let _ = relay
                        .send(Message::ExploreProgress(reporting.clone(), fraction))
                        .await;
                }
            });

            let outcome = fetch_and_file(&found, &into, &root, &progress).await;
            drop(progress);

            let _ = sender
                .send(Message::ExploreDownloaded(
                    found.id.clone(),
                    Box::new(outcome),
                ))
                .await;
        },
    ))
}

#[cfg(feature = "explore")]
async fn fetch_and_file(
    found: &Found,
    into: &Destination,
    root: &std::path::Path,
    progress: &tokio::sync::mpsc::Sender<f32>,
) -> Result<PathBuf, String> {
    let fetcher = verse_core::explore::YtDlp::new();
    let staging = root.join(".verse-downloads");

    let audio = fetcher
        .fetch(&found.id, &staging, |update| {
            if let Some(fraction) = update.fraction {
                let _ = progress.try_send(fraction);
            }
        })
        .await
        .map_err(|e| e.to_string())?;

    let mut into = into.clone();
    into.cover = match found.cover_url.as_deref() {
        Some(url) => verse_core::explore::fetch_cover_for_file(url).await,
        None => None,
    };

    verse_core::explore::write_tags(&audio, found, &into).map_err(|e| e.to_string())?;

    let filed = verse_core::explore::path_for(root, found, &into);
    if let Some(parent) = filed.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }

    tokio::fs::rename(&audio, &filed)
        .await
        .map_err(|e| e.to_string())?;

    Ok(filed)
}

#[cfg(feature = "explore")]
pub fn fetch_art(url: String) -> Task<Message> {
    Task::future(async move {
        let bytes = verse_core::explore::fetch_cover(&url).await;
        Message::ExploreArt(url, bytes.map(iced::widget::image::Handle::from_bytes))
    })
}
