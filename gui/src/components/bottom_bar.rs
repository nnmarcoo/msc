use iced::alignment::{Horizontal, Vertical};
use iced::widget::svg::Handle;
use iced::widget::{column, container, row, space, svg, text, tooltip};
use iced::{Element, Length, Theme};

use crate::styles::{BAR_HEIGHT, PAD, bar_style, svg_style};
use crate::widgets::canvas_button::canvas_button;
use crate::widgets::menu::{menu_item, menu_separator, styled_menu};
use crate::widgets::menu_button::MenuButton;

#[derive(Debug, Clone)]
pub enum Message {
    ClearQueue,
    ToggleEditMode,
    OpenPreferences,
    SwitchPreset(usize),
    AddPreset,
    RemovePreset,
}

pub fn view(
    preset_count: usize,
    _current_preset: usize,
    edit_mode: bool,
) -> Element<'static, Message> {
    let mut preset_buttons = row![].spacing(2).align_y(Vertical::Center);

    if preset_count > 1 || edit_mode {
        for index in 0..preset_count {
            preset_buttons = preset_buttons.push(
                canvas_button(
                    text((index + 1).to_string())
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center),
                )
                .width(20)
                .height(20)
                .on_press(Message::SwitchPreset(index)),
            );
        }
    }

    if edit_mode {
        if preset_count < 10 {
            preset_buttons = preset_buttons.push(
                canvas_button(
                    svg(Handle::from_memory(include_bytes!(
                        "../../../assets/icons/plus.svg"
                    )))
                    .style(svg_style),
                )
                .width(20)
                .height(20)
                .on_press(Message::AddPreset),
            );
        }
        if preset_count > 1 {
            preset_buttons = preset_buttons.push(
                canvas_button(
                    svg(Handle::from_memory(include_bytes!(
                        "../../../assets/icons/minus.svg"
                    )))
                    .style(svg_style),
                )
                .width(20)
                .height(20)
                .on_press(Message::RemovePreset),
            );
        }
    }

    let right_side: Element<'static, Message> = if edit_mode {
        tooltip(
            canvas_button(
                svg(Handle::from_memory(include_bytes!(
                    "../../../assets/icons/checkmark.svg"
                )))
                .style(svg_style),
            )
            .width(20)
            .height(20)
            .on_press(Message::ToggleEditMode),
            container(text("Done").size(12))
                .padding(6)
                .style(container::rounded_box),
            tooltip::Position::Top,
        )
        .into()
    } else {
        row![
            canvas_button(
                text("clear")
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center),
            )
            .height(20)
            .on_press(Message::ClearQueue),
            MenuButton::new(
                include_bytes!("../../../assets/icons/kebab.svg"),
                styled_menu(column![
                    menu_item("Edit Layout", Message::ToggleEditMode),
                    menu_separator(),
                    menu_item("Preferences", Message::OpenPreferences),
                ]),
            ),
        ]
        .spacing(2)
        .align_y(Vertical::Center)
        .into()
    };

    container(
        row![preset_buttons, space().width(Length::Fill), right_side,]
            .height(Length::Fixed(BAR_HEIGHT))
            .width(Length::Fill)
            .align_y(Vertical::Center)
            .spacing(PAD),
    )
    .padding([0.0, PAD])
    .style(bar_style)
    .into()
}
