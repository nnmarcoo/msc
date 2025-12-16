use iced::{
    Theme,
    window::{Settings, icon::from_file_data},
};

mod app;
mod components;
mod pane;
mod widgets;

use app::App;

pub fn main() -> iced::Result {
    iced::application("msc", App::update, App::view)
        .window(Settings {
            icon: from_file_data(include_bytes!("../../assets/logo.png"), None).ok(),
            ..Default::default()
        })
        .centered()
        .subscription(App::subscription)
        .theme(|_| Theme::CatppuccinFrappe)
        .run()
}
