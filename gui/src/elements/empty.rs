use iced::Element;
use iced::widget::Space;

use crate::app::Message;

pub fn view<'a>() -> Element<'a, Message> {
    Space::new(0, 0).into()
}
