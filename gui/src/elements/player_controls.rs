use iced::widget::{column, text};
use iced::Element;

use crate::layout::Message;

pub fn view<'a>() -> Element<'a, Message> {
    column![
        text("▶ Play / ⏸ Pause").size(20),
        text("⏮ Previous / ⏭ Next").size(20),
        text("Volume Control").size(16),
    ]
    .spacing(10)
    .padding(20)
    .into()
}
