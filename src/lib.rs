#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::Msc;

mod components;
mod core;
pub use core::resize;
pub use core::structs;
