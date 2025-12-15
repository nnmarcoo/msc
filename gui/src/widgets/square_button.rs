use iced::Length;
use iced::widget::button;

use super::sharp_button::sharp_button;

pub fn square_button<'a, Message: 'a>(
    content: impl Into<iced::Element<'a, Message>>,
    size: impl Into<Length>,
) -> button::Button<'a, Message> {
    let size = size.into();
    sharp_button(content).width(size).height(size)
}
