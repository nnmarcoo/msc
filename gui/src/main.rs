use iced::Theme;

mod app;
mod components;
mod pane;
mod widgets;

use app::App;

pub fn main() -> iced::Result {
    iced::application("msc", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| Theme::CatppuccinFrappe)
        .run()
}
