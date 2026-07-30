//! Cover art: what is held, and how pixels are produced.
//!
//! [`cache`] owns what is kept and at which sizes; [`decode`] turns a file into
//! pixels at one of them. They are split because only the first is state: the
//! cache decides *what* to draw and is read every frame, while decoding is a pure
//! function of a file and a size, run on a blocking thread and never touching the
//! cache itself. [`crate::tasks`] is what joins them, and holds neither.

pub mod cache;
pub mod decode;

pub use cache::{Art, ArtKey, Cache, Job, Source};
pub use decode::{Decoded, decode};
