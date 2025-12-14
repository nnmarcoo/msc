use iced::widget::{button, text};
use iced::{Element, Theme, Border, Color};

pub fn link_button<'a, Message: Clone + 'a>(
    label: &'a str,
) -> button::Button<'a, Message> {
    button(text(label))
        .padding([2, 0])
        .style(move |theme: &Theme, status: button::Status| {
            let palette = theme.extended_palette();
            
            button::Style {
                background: None,
                text_color: match status {
                    button::Status::Active => palette.primary.strong.color,
                    button::Status::Hovered => palette.primary.base.color,
                    button::Status::Pressed => palette.primary.weak.color,
                    button::Status::Disabled => palette.background.strong.color,
                },
                border: Border {
                    color: match status {
                        button::Status::Hovered => palette.primary.base.color,
                        _ => Color::TRANSPARENT,
                    },
                    width: match status {
                        button::Status::Hovered => 1.0,
                        _ => 0.0,
                    },
                    radius: 0.0.into(),
                },
                shadow: Default::default(),
            }
        })
}
