use iced::alignment::{Horizontal, Vertical};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::tooltip::Position;
use iced::widget::{button, column, container, row, rule, scrollable, svg, text, toggler, tooltip};
use iced::{Element, Length, Theme};

use crate::config::{Config, PresetIndicator};
use crate::styles::{PAD, TOOLTIP_DELAY, bar_style, svg_style};
use crate::widgets::canvas_button::canvas_button;
use crate::widgets::theme_picker::ThemePicker;

#[derive(Debug, Clone)]
pub enum PreferenceMessage {
    SetTheme(Theme),
    SetRounded(bool),
    SetPresetIndicator(PresetIndicator),
    SetLibrary,
    Reset,
    Save,
    Cancel,
}

fn section<'a>(label: &'a str, theme: &Theme) -> Element<'a, PreferenceMessage> {
    let accent = theme.extended_palette().primary.base.color;
    column![text(label).size(11).color(accent), rule::horizontal(1),]
        .spacing(PAD)
        .into()
}

fn setting<'a>(
    label: &'a str,
    description: &'a str,
    control: Element<'a, PreferenceMessage>,
    theme: &Theme,
) -> Element<'a, PreferenceMessage> {
    let muted = theme
        .extended_palette()
        .background
        .base
        .text
        .scale_alpha(0.5);
    row![
        column![
            text(label).size(13),
            text(description).size(11).color(muted),
        ]
        .spacing(PAD / 2.0)
        .width(Length::Fill),
        control,
    ]
    .align_y(Vertical::Center)
    .spacing(PAD * 2.0)
    .into()
}

pub fn view<'a>(pending: &'a Config, theme: &Theme) -> Element<'a, PreferenceMessage> {
    let action_buttons = container(
        row![
            tooltip(
                button(text("Reset").size(12))
                    .on_press(PreferenceMessage::Reset)
                    .padding([4.0, 8.0]),
                container(text("Reset All Settings To Defaults").size(12))
                    .padding(6)
                    .style(container::rounded_box),
                Position::Top,
            )
            .gap(8)
            .delay(TOOLTIP_DELAY)
            .snap_within_viewport(true),
            iced::widget::Space::new().width(Length::Fill),
            tooltip(
                canvas_button(
                    svg(SvgHandle::from_memory(include_bytes!(
                        "../../../assets/icons/checkmark.svg"
                    )))
                    .width(20)
                    .height(20)
                    .style(svg_style),
                )
                .width(20)
                .height(20)
                .on_press(PreferenceMessage::Save),
                container(text("Save").size(12))
                    .padding(6)
                    .style(container::rounded_box),
                Position::Top,
            )
            .gap(8)
            .delay(TOOLTIP_DELAY)
            .snap_within_viewport(true),
            tooltip(
                canvas_button(
                    svg(SvgHandle::from_memory(include_bytes!(
                        "../../../assets/icons/x.svg"
                    )))
                    .width(20)
                    .height(20)
                    .style(svg_style),
                )
                .width(20)
                .height(20)
                .on_press(PreferenceMessage::Cancel),
                container(text("Cancel").size(12))
                    .padding(6)
                    .style(container::rounded_box),
                Position::Top,
            )
            .gap(8)
            .delay(TOOLTIP_DELAY)
            .snap_within_viewport(true),
        ]
        .align_y(Vertical::Center)
        .spacing(PAD),
    )
    .width(Length::Fill)
    .padding(PAD * 2.0)
    .style(bar_style);

    let content = column![
        container(text("Preferences").size(16))
            .width(Length::Fill)
            .align_x(Horizontal::Center),
        iced::widget::Space::new().height(PAD * 2.0),
        section("Appearance", theme),
        iced::widget::Space::new().height(PAD),
        setting(
            "Theme",
            "Color scheme for the application",
            ThemePicker::new(pending.theme.clone(), PreferenceMessage::SetTheme).into(),
            theme,
        ),
        iced::widget::Space::new().height(PAD),
        setting(
            "Rounded corners",
            "Use rounded corners on UI elements",
            toggler(pending.rounded)
                .on_toggle(PreferenceMessage::SetRounded)
                .into(),
            theme,
        ),
        iced::widget::Space::new().height(PAD),
        setting(
            "Layout indicators",
            "Show layout presets as numbers or dots",
            toggler(pending.preset_indicator == PresetIndicator::Dots)
                .on_toggle(|dots| {
                    PreferenceMessage::SetPresetIndicator(if dots {
                        PresetIndicator::Dots
                    } else {
                        PresetIndicator::Numbers
                    })
                })
                .into(),
            theme,
        ),
        iced::widget::Space::new().height(PAD * 2.0),
        section("Library", theme),
        iced::widget::Space::new().height(PAD),
        setting(
            "Music library folder",
            "The folder scanned for your music collection",
            button(text("Set Folder").size(12))
                .on_press(PreferenceMessage::SetLibrary)
                .padding([4.0, 8.0])
                .into(),
            theme,
        ),
    ]
    .spacing(PAD)
    .padding(PAD * 3.0)
    .width(Length::Fill);

    column![
        scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(Direction::Vertical(
                Scrollbar::new().width(4).scroller_width(4),
            )),
        rule::horizontal(1),
        action_buttons,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
