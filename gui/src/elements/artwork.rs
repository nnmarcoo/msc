use iced::widget::{column, container, text};
use iced::{Element, Length};

use crate::layout::Message;

pub fn view<'a>() -> Element<'a, Message> {
    column![
        container(text("🎵 Album Art").size(32))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
    ]
    .into()
}
