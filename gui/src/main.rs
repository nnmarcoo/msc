#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::{
    Size,
    window::{Settings, icon::from_file_data},
};

mod app;
mod art_cache;
mod components;
mod config;
mod formatters;
mod image_processing;
mod media_controls;
mod pane;
mod pane_view;
mod panes;
mod styles;
mod widgets;
mod window_handle;

use app::App;

pub fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("msc")
        .window(Settings {
            min_size: Some(Size::new(300., 0.)),
            icon: from_file_data(include_bytes!("../../assets/logo.png"), None).ok(),
            exit_on_close_request: false,
            ..Default::default()
        })
        .centered()
        .subscription(App::subscription)
        .theme(|app: &App| app.theme())
        .run()
}
