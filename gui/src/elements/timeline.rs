use iced::widget::{column, text};
use iced::Element;

use crate::layout::Message;

pub fn view<'a>() -> Element<'a, Message> {
    column![
        text("Timeline / Seek Bar").size(16),
        text("0:00 ━━━━━━━━━━ 3:45").size(14),
    ]
    .spacing(10)
    .padding(20)
    .into()
}
