#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod components;
mod config;
mod layout;
mod pane;
mod styles;
mod tasks;

use app::App;
use iced::{Size, window};

fn main() -> iced::Result {
    let config = config::Config::load();

    iced::application(move || App::new(config.clone()), App::update, App::view)
        .title(App::title)
        .window(window::Settings {
            min_size: Some(Size::new(640.0, 360.0)),
            ..Default::default()
        })
        .centered()
        .theme(App::theme)
        .subscription(App::subscription)
        .run()
}
