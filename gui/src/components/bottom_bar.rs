use iced::alignment::{Horizontal, Vertical};
use iced::widget::svg::Handle;
use iced::widget::{container, horizontal_space, row, svg, text};
use iced::{Background, Element, Length, Theme};

use crate::widgets::canvas_button::canvas_button;

#[derive(Debug, Clone)]
pub enum Message {
    LoadLibrary,
    QueueLibrary,
    ToggleEditMode,
    SwitchPreset(usize),
    AddPreset,
}

pub fn view(
    preset_count: usize,
    _current_preset: usize,
    edit_mode: bool,
) -> Element<'static, Message> {
    let mut preset_buttons = row![].spacing(5).align_y(Vertical::Center);

    for index in 0..preset_count {
        let btn = canvas_button(
            text((index + 1).to_string())
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
        )
        .width(20)
        .height(20)
        .on_press(Message::SwitchPreset(index));

        preset_buttons = preset_buttons.push(btn);
    }

    if edit_mode {
        preset_buttons = preset_buttons.push(
            canvas_button(svg(Handle::from_memory(include_bytes!(
                "../../../assets/icons/plus.svg"
            ))))
            .width(20)
            .height(20)
            .on_press(Message::AddPreset),
        );
    }

    let bottom_bar = if edit_mode {
        container(
            row![
                preset_buttons,
                horizontal_space(),
                canvas_button(svg(Handle::from_memory(include_bytes!(
                    "../../../assets/icons/checkmark.svg"
                ))))
                .width(20)
                .height(20)
                .on_press(Message::ToggleEditMode)
            ]
            .spacing(5)
            .align_y(Vertical::Center),
        )
        .width(Length::Fill)
        .style(bar_style)
    } else {
        container(
            row![
                preset_buttons,
                horizontal_space(),
                canvas_button(
                    text("loadlib")
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center)
                )
                .height(20)
                .on_press(Message::LoadLibrary),
                canvas_button(
                    text("quelib")
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center)
                )
                .height(20)
                .on_press(Message::QueueLibrary),
                canvas_button(svg(Handle::from_memory(include_bytes!(
                    "../../../assets/icons/settings.svg"
                ))))
                .width(20)
                .height(20)
                .on_press(Message::ToggleEditMode),
            ]
            .spacing(5),
        )
        .width(Length::Fill)
        .style(bar_style)
    };

    bottom_bar.into()
}

fn bar_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    let mut color = palette.background.base.color;
    color.r = (color.r + 0.02).min(1.0);
    color.g = (color.g + 0.02).min(1.0);
    color.b = (color.b + 0.02).min(1.0);
    container::Style {
        text_color: Some(palette.background.base.text),
        background: Some(Background::Color(color)),
        ..Default::default()
    }
}
