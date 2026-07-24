//! Async side effects: the folder picker and off-thread library scan.

use std::path::PathBuf;

use iced::Task;
use verse_core::Library;

use crate::app::Message;

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
