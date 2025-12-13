use iced::widget::{column, text};
use iced::{Element, Theme};

use crate::app::Message;

pub fn view<'a>() -> Element<'a, Message> {
    column![
        text("Timeline / Seek Bar")
            .size(16)
            .style(|theme: &Theme| text::Style {
                color: Some(theme.extended_palette().background.base.text),
            }),
        text("0:00 ━━━━━━━━━━ 3:45")
            .size(14)
            .style(|theme: &Theme| text::Style {
                color: Some(theme.extended_palette().background.base.text),
            }),
    ]
    .spacing(10)
    .padding(20)
    .into()
}
