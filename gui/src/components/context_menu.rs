use blake3::Hash;
use iced::alignment::Vertical;
use iced::widget::{column, container, text};
use iced::{Background, Border, Element, Length, Shadow, Theme};
use iced_aw::ContextMenu;

use crate::widgets::canvas_button::canvas_button;

pub fn track_context_menu<'a, Message: 'a + Clone>(
    content: impl Into<Element<'a, Message>>,
    _track_id: Hash,
    play_msg: Message,
    queue_msg: Message,
    queue_next_msg: Message,
) -> Element<'a, Message> {
    ContextMenu::new(content, move || {
        container(
            column![
                canvas_button(text(" Play").align_y(Vertical::Center))
                    .width(Length::Fill)
                    .height(28)
                    .on_press(play_msg.clone()),
                canvas_button(text(" Queue Next").align_y(Vertical::Center))
                    .width(Length::Fill)
                    .height(28)
                    .on_press(queue_next_msg.clone()),
                canvas_button(text(" Queue").align_y(Vertical::Center))
                    .width(Length::Fill)
                    .height(28)
                    .on_press(queue_msg.clone()),
            ]
            .spacing(2),
        )
        .width(Length::Fixed(140.0))
        .padding(6)
        .style(menu_container_style)
        .into()
    })
    .into()
}

fn menu_container_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    let mut color = palette.background.base.color;
    color.r = (color.r + 0.03).min(1.0);
    color.g = (color.g + 0.03).min(1.0);
    color.b = (color.b + 0.03).min(1.0);
    container::Style {
        text_color: Some(palette.background.base.text),
        background: Some(Background::Color(color)),
        border: Border::default(),
        shadow: Shadow {
            color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.2),
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 6.0,
        },
    }
}
