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
//!
//! Download progress goes over a `watch` channel rather than an `mpsc`. A
//! progress bar wants the newest fraction and nothing else — an intermediate one
//! is worthless the moment a later one exists — which is exactly what `watch`
//! keeps. The bounded `mpsc` it replaced dropped sends once full, so a burst of
//! `yt-dlp` output left bars frozen at whatever fraction last got through, and a
//! bar stuck at 87% reads as a hung download rather than a busy one.
//!
//! The completing `1.0` is sent explicitly rather than relied upon from the
//! output. `yt-dlp` reports the last chunk before its own remux and fixup, so
//! the final line is not always 100%, and the row must not settle on whatever
//! number happened to come last.
//!
//! A download resolves its id before fetching it. A listing's id does not always
//! name the recording the listing described — see [`verse_core::explore`] — and
//! when it does not, the album version is fetched in its place. That is a
//! substitution the user did not ask for, so it is reported rather than made
//! silently, on its own message ahead of the download's own result.

use std::path::PathBuf;

use iced::Task;
use verse_core::Library;

use crate::app::Message;
use crate::artwork::{Decoded, Job, Source, decode};

#[cfg(feature = "explore")]
use std::sync::Arc;
#[cfg(feature = "explore")]
use verse_core::explore::{Destination, DownloadSource, Found, Innertube, MusicSource, YtDlp};

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
pub fn check_fetcher(fetcher: Arc<YtDlp>) -> Task<Message> {
    Task::future(async move { Message::FetcherChecked(fetcher.available().await) })
}

#[cfg(feature = "explore")]
pub fn download(
    source: Arc<Innertube>,
    fetcher: Arc<YtDlp>,
    found: Found,
    into: Destination,
    root: PathBuf,
) -> Task<Message> {
    let id = found.id.clone();

    Task::stream(iced::stream::channel(
        16,
        move |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
            let (progress, mut updates) = tokio::sync::watch::channel(0.0_f32);
            let reporting = id.clone();

            let mut relay = sender.clone();
            let relaying = tokio::spawn(async move {
                while updates.changed().await.is_ok() {
                    let fraction = *updates.borrow_and_update();
                    let _ = relay
                        .send(Message::ExploreProgress(reporting.clone(), fraction))
                        .await;
                }
            });

            let outcome = fetch_and_file(&source, &fetcher, &found, &into, &root, &progress).await;
            drop(progress);
            let _ = relaying.await;

            let (outcome, substituted) = match outcome {
                Ok((filed, substituted)) => (Ok(filed), substituted),
                Err(e) => (Err(e), None),
            };

            if substituted.is_some() {
                let _ = sender
                    .send(Message::ExploreSubstituted(
                        found.id.clone(),
                        found.title.clone(),
                    ))
                    .await;
            }

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
    source: &Innertube,
    fetcher: &YtDlp,
    found: &Found,
    into: &Destination,
    root: &std::path::Path,
    progress: &tokio::sync::watch::Sender<f32>,
) -> Result<(PathBuf, Option<String>), String> {
    let staging = staging_directory();

    let resolved = verse_core::explore::for_download(source, fetcher, found).await;

    let audio = fetcher
        .fetch(&resolved.id, &staging, |update| {
            if let Some(fraction) = update.fraction {
                let _ = progress.send(fraction);
            }
        })
        .await
        .map_err(|e| e.to_string())?;

    let _ = progress.send(1.0);

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

    file_into_place(&audio, &filed).await?;

    Ok((filed, resolved.substituted))
}

#[cfg(feature = "explore")]
fn staging_directory() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("verse")
        .join("downloads")
}

#[cfg(feature = "explore")]
async fn file_into_place(from: &std::path::Path, to: &PathBuf) -> Result<(), String> {
    match tokio::fs::rename(from, to).await {
        Ok(()) => return Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {}
        Err(e) => return Err(e.to_string()),
    }

    tokio::fs::copy(from, to).await.map_err(|e| e.to_string())?;
    tokio::fs::remove_file(from).await.ok();

    Ok(())
}

#[cfg(feature = "explore")]
pub fn fetch_art(url: String) -> Task<Message> {
    Task::future(async move {
        let bytes = verse_core::explore::fetch_cover(&url).await;
        Message::ExploreArt(url, bytes.map(iced::widget::image::Handle::from_bytes))
    })
}
