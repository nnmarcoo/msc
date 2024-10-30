#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod components;
mod msc;
mod util;

use eframe::{egui::ViewportBuilder, run_native, Error, NativeOptions, Result};
use msc::Msc;

fn main() -> Result<(), Error> {
    let native_options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("msc")
            .with_decorations(false)
            .with_min_inner_size([400., 300.]),
        ..Default::default()
    };
    run_native(
        "msc",
        native_options,
        Box::new(|cc| Ok(Box::new(Msc::new(cc)))),
    )
}
