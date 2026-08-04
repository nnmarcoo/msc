//! Cover art: what is held, and how pixels are produced.
//!
//! [`cache`] owns what is kept and at which sizes; [`decode`] turns a file into
//! pixels at one of them. They are split because only the first is state: the
//! cache decides *what* to draw and is read every frame, while decoding is a pure
//! function of a file and a size, run on a blocking thread and never touching the
//! cache itself. [`crate::tasks`] is what joins them, and holds neither.
//!
//! [`palette`] is a third pure function, over an image rather than a file: the
//! color a cover is mostly made of, for surfaces that want to be tinted by the
//! record they show. It runs on the decode thread beside the resampling, since
//! it wants the same master and costs a fraction of what the resample does.
//!
//! [`accent`] then turns that color into one a pane can draw with, which is a
//! separate question: what [`palette`] names is fitted to sit behind text, and a
//! rail or a heading drawn in it directly would be far too dark. Panes tinting
//! themselves by the record go through it rather than reading a cover color
//! straight, so they all agree about what the record's color *is*.

pub mod accent;
pub mod cache;
pub mod decode;
pub mod palette;
#[cfg(feature = "explore")]
pub mod remote;

pub use cache::{Art, ArtKey, Cache, Job, Source};
pub use decode::{Decoded, decode};
#[cfg(feature = "explore")]
pub use remote::Remote;
