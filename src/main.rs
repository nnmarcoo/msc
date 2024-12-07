#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backend;
mod components;
mod constants;
mod msc;
mod widgets;

use eframe::{
    egui::{IconData, ViewportBuilder},
    run_native, Error, NativeOptions, Result,
};
use image::load_from_memory;
use msc::Msc;

fn main() -> Result<(), Error> {
    let native_options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("msc")
            .with_decorations(false)
            .with_min_inner_size([1000., 625.])
            .with_icon(load_icon()),
        ..Default::default()
    };
    run_native(
        "msc",
        native_options,
        Box::new(|cc| Ok(Box::new(Msc::new(cc)))),
    )
}

fn load_icon() -> IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = include_bytes!("../assets/icons/logo.png");
        let image = load_from_memory(icon)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}
