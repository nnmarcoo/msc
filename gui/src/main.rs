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
mod pane_view;
mod panes;
mod widgets;
mod window_handle;

use app::App;

pub fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .window(Settings {
            min_size: Some(Size::new(300., 0.)),
            icon: from_file_data(include_bytes!("../../assets/logo.png"), None).ok(),
            ..Default::default()
        })
        .centered()
        .subscription(App::subscription)
        .theme(|_: &App| Theme::KanagawaDragon)
        .run()
}
