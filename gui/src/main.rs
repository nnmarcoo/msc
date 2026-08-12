#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod artwork;
mod browsing;
mod config;
#[cfg(feature = "explore")]
mod download;
mod keybinds;
mod layout;
mod pane;
mod preferences;
mod styles;
mod tasks;
mod widgets;

use app::App;
use iced::{Size, window};

fn main() -> iced::Result {
    let config = config::Config::load();

    iced::application(move || App::new(config.clone()), App::update, App::view)
        .title(App::title)
        .window(window::Settings {
            min_size: Some(Size::new(200.0, 200.0)),
            ..Default::default()
        })
        .centered()
        .theme(App::theme)
        .subscription(App::subscription)
        .run()
}
