use iced::widget::Space;
use iced::Element;

use crate::layout::Message;

pub fn view<'a>() -> Element<'a, Message> {
    Space::new(0, 0).into()
}
