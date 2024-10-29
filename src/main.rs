#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod msc;

use eframe::{egui::ViewportBuilder, run_native, Error, NativeOptions, Result};
use msc::Msc;

fn main() -> Result<(), Error> {
    let native_options = NativeOptions {
        viewport: ViewportBuilder::default()
        .with_title("msc"),
        ..Default::default()
    };
    run_native(
        "msc",
        native_options,
        Box::new(|cc| Ok(Box::new(Msc::new(cc)))),
    )
}
