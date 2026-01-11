use iced::Element;
use iced::widget::space;

use crate::app::Message;

pub fn view<'a>() -> Element<'a, Message> {
    space().into()
}
