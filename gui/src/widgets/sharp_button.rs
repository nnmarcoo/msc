use iced::widget::button;
use iced::{Element, Theme};

pub fn sharp_button<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> button::Button<'a, Message> {
    button(content)
        .padding(0)
        .style(|theme: &Theme, status: button::Status| {
            let base = button::primary(theme, status);
            button::Style {
                background: match status {
                    button::Status::Hovered | button::Status::Pressed => base.background,
                    _ => None,
                },
                border: iced::Border {
                    radius: 0.0.into(),
                    ..base.border
                },
                ..base
            }
        })
}
