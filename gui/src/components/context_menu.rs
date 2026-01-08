use iced::alignment::Vertical;
use iced::widget::{column, container, text};
use iced::{Background, Border, Color, Element, Length, Shadow, Theme};
use iced_aw::ContextMenu;

use crate::widgets::canvas_button::canvas_button;

pub enum MenuElement<Message> {
    Button { label: String, message: Message },
    Separator,
}

impl<Message: Clone> MenuElement<Message> {
    pub fn button(label: impl Into<String>, message: Message) -> Self {
        Self::Button {
            label: label.into(),
            message,
        }
    }

    pub fn separator() -> Self {
        Self::Separator
    }
}

pub fn context_menu<'a, Message: 'a + Clone>(
    content: impl Into<Element<'a, Message>>,
    items: Vec<MenuElement<Message>>,
    width: Length,
) -> Element<'a, Message> {
    ContextMenu::new(content, move || {
        let menu_column = items.iter().fold(column![].spacing(2), |col, item| {
            let element: Element<'a, Message> = match item {
                MenuElement::Button { label, message } => {
                    canvas_button(text(format!(" {}", label)).align_y(Vertical::Center))
                        .width(Length::Fill)
                        .height(28)
                        .on_press(message.clone())
                        .into()
                }
                MenuElement::Separator => container(text(""))
                    .width(Length::Fill)
                    .height(1)
                    .style(separator_style)
                    .into(),
            };
            col.push(element)
        });

        container(menu_column)
            .width(width)
            .padding(6)
            .style(menu_container_style)
            .into()
    })
    .into()
}

fn separator_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        ..Default::default()
    }
}

fn menu_container_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    let base = palette.background.base.color;

    let color = Color {
        r: (base.r + 0.03).min(1.0),
        g: (base.g + 0.03).min(1.0),
        b: (base.b + 0.03).min(1.0),
        ..base
    };

    container::Style {
        text_color: Some(palette.background.base.text),
        background: Some(Background::Color(color)),
        border: Border::default(),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 6.0,
        },
    }
}
