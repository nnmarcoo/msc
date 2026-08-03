//! Writing tags and cover art onto a downloaded file.
//!
//! A download arrives as audio with no metadata, and verse's library is built
//! entirely from tags — an untagged file would scan as a track with no title,
//! no artist and no album, which is to say invisible in every pane that groups
//! by any of them. Tagging is therefore part of downloading rather than a step
//! after it.
//!
//! Filled in at step 3 of the plan.

use std::path::Path;

use thiserror::Error;

use super::{Destination, Found};

#[derive(Debug, Error)]
pub enum TagError {
    #[error("Could not read {0} to tag it")]
    Unreadable(String),
    #[error("Could not write tags: {0}")]
    Write(String),
}

pub fn write_tags(_path: &Path, _found: &Found, _into: &Destination) -> Result<(), TagError> {
    Err(TagError::Write("not implemented yet".to_owned()))
}
