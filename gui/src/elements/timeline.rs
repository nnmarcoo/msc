use iced::Element;
use iced::widget::{column, text};

use crate::app::Message;

pub fn view<'a>() -> Element<'a, Message> {
    column![
        text("Timeline / Seek Bar").size(16),
        text("0:00 ━━━━━━━━━━ 3:45").size(14),
    ]
    .spacing(10)
    .padding(20)
    .into()
}
