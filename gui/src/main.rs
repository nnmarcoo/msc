#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::{
    Size, Theme,
    window::{Settings, icon::from_file_data},
};

mod app;
mod components;
mod formatters;
mod media_controls;
mod pane;
mod widgets;
mod window_handle;

use app::App;

pub fn main() -> iced::Result {
    iced::application("msc", App::update, App::view)
        .window(Settings {
            min_size: Some(Size::new(300., 0.)),
            icon: from_file_data(include_bytes!("../../assets/logo.png"), None).ok(),
            ..Default::default()
        })
        .centered()
        .subscription(App::subscription)
        .theme(|_| Theme::KanagawaDragon)
        .run()
}
