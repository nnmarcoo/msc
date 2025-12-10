use iced::widget::{column, text};
use iced::Element;

use crate::layout::Message;

pub fn view<'a>() -> Element<'a, Message> {
    column![
        text("Library Browser").size(18),
        text("Albums / Artists / Tracks").size(14),
    ]
    .spacing(10)
    .padding(20)
    .into()
}
