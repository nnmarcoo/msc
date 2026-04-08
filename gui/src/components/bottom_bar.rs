use iced::alignment::{Horizontal, Vertical};
use iced::widget::svg::Handle;
use iced::widget::{column, container, row, space, svg, text, tooltip};
use iced::{Element, Length};

use crate::config::PresetIndicator;
use crate::styles::{BAR_HEIGHT, PAD, TOOLTIP_DELAY, bar_style, svg_style};
use crate::widgets::canvas_button::canvas_button;
use crate::widgets::menu::{menu_item, menu_separator, styled_menu};
use crate::widgets::menu_button::MenuButton;

#[derive(Debug, Clone)]
pub enum Message {
    ToggleEditMode,
    OpenPreferences,
    SwitchPreset(usize),
    AddPreset,
    RemovePreset,
}

pub fn view(
    preset_count: usize,
    current_preset: usize,
    edit_mode: bool,
    preset_indicator: PresetIndicator,
) -> Element<'static, Message> {
    let mut preset_buttons = row![].spacing(2).align_y(Vertical::Center);

    if preset_count > 1 || edit_mode {
        for index in 0..preset_count {
            let label = match preset_indicator {
                PresetIndicator::Numbers => (index + 1).to_string(),
                PresetIndicator::Dots => "•".to_string(),
            };
            preset_buttons = preset_buttons.push(
                canvas_button(
                    text(label)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center),
                )
                .width(20)
                .height(20)
                .active(index == current_preset)
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
        .gap(8)
        .delay(TOOLTIP_DELAY)
        .snap_within_viewport(true)
        .into()
    } else {
        row![MenuButton::new(
            include_bytes!("../../../assets/icons/kebab.svg"),
            styled_menu(column![
                menu_item("Edit Layout", Message::ToggleEditMode),
                menu_separator(),
                menu_item("Preferences", Message::OpenPreferences),
            ]),
        ),]
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
