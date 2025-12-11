use iced::Theme;

mod app;
mod elements;
mod pane;

use app::App;

pub fn main() -> iced::Result {
    iced::application("MSC - Music Player", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| Theme::Dark)
        .run()
}
