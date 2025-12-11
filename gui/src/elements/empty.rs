use iced::Element;
use iced::widget::{column, text};

use crate::layout::Message;

pub fn view<'a>() -> Element<'a, Message> {
    column![text("Empty Pane").size(14)]
        .spacing(5)
        .padding(20)
        .into()
}
