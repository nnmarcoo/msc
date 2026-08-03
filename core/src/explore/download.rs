//! Fetching audio via the `yt-dlp` binary.
//!
//! Resolving a YouTube stream URL means solving a signature challenge that
//! YouTube changes deliberately and often. `yt-dlp` ships releases at a cadence
//! nothing in this repository could match, so it is invoked rather than
//! reimplemented, and it is detected on PATH rather than bundled: a binary
//! whose whole value is being current should be updated by whoever installed
//! it.
//!
//! Filled in at step 4 of the plan; the types exist now so the pane and the
//! queue above it can be written against them.

use std::path::{Path, PathBuf};

use thiserror::Error;

use super::DownloadSource;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("yt-dlp was not found on PATH")]
    NotInstalled,
    #[error("yt-dlp failed: {0}")]
    Failed(String),
    #[error("IO error: {0}")]
    Io(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    pub fraction: Option<f32>,
}

pub struct YtDlp {
    binary: PathBuf,
}

impl Default for YtDlp {
    fn default() -> Self {
        Self::new()
    }
}

impl YtDlp {
    pub fn new() -> Self {
        Self {
            binary: PathBuf::from("yt-dlp"),
        }
    }

    pub fn at(binary: PathBuf) -> Self {
        Self { binary }
    }

    pub fn binary(&self) -> &Path {
        &self.binary
    }
}

impl DownloadSource for YtDlp {
    async fn fetch(
        &self,
        _id: &str,
        _directory: &Path,
        _progress: impl FnMut(Progress) + Send,
    ) -> Result<PathBuf, DownloadError> {
        Err(DownloadError::Failed("not implemented yet".to_owned()))
    }
}
