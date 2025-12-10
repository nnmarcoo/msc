use iced::widget::{column, text};
use iced::Element;

use crate::layout::Message;

pub fn view<'a>() -> Element<'a, Message> {
    column![
        text("Track 1 - Artist Name").size(14),
        text("Track 2 - Artist Name").size(14),
        text("Track 3 - Artist Name").size(14),
    ]
    .spacing(5)
    .padding(20)
    .into()
}
