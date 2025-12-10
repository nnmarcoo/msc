use iced::Theme;

mod elements;
mod layout;
mod pane;

use layout::Layout;

pub fn main() -> iced::Result {
    iced::application("MSC - Music Player", Layout::update, Layout::view)
        .subscription(Layout::subscription)
        .theme(|_| Theme::Dark)
        .run()
}
