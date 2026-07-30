//! Async side effects: the folder picker, the library scan, and artwork decoding.
//!
//! Each is a `Task` that ends in one [`Message`], so `update` stays the only
//! place state changes. The work itself lives elsewhere — scanning in
//! [`verse_core::Library`], decoding in [`crate::artwork::decode`] — and what is
//! here is only the moving of it off the frame thread and the naming of what
//! comes back. Anything that can fail answers with a message rather than
//! unwinding, since a task that dies silently leaves a pane waiting forever.

use std::path::PathBuf;

use iced::Task;
use verse_core::Library;

use crate::app::Message;
use crate::artwork::{Decoded, Job, Source, decode};

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
