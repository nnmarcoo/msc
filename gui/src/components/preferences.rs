use iced::alignment::{Horizontal, Vertical};
use iced::font::Weight;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::tooltip::Position;
use iced::widget::{
    Space, button, column, container, row, rule, scrollable, svg, text, toggler, tooltip,
};
use iced::{Element, Font, Length, Theme};

use crate::config::{Config, PresetIndicator};
use crate::styles::{
    BAR_HEIGHT, PAD, PREF_CONTENT_MAX_WIDTH, PREF_SIDEBAR_WIDTH, RULE_HEIGHT, TOOLTIP_DELAY,
    bar_style, panel_divider_style, plain_icon_button_style, pref_nav_button_style,
    pref_section_rule_style, svg_style,
};
use crate::widgets::canvas_button::canvas_button;
use crate::widgets::theme_picker::ThemePicker;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefSection {
    #[default]
    Appearance,
    Library,
}

#[derive(Debug, Clone)]
pub enum PreferenceMessage {
    SelectSection(PrefSection),
    SetTheme(Theme),
    SetRounded(bool),
    SetPresetIndicator(PresetIndicator),
    SetLibrary,
    ResetAppearance,
    Reset,
    Save,
    Cancel,
    ClearLibrary,
    ConfirmClearLibrary,
    CancelClearLibrary,
}

fn with_tooltip<'a>(
    content: impl Into<Element<'a, PreferenceMessage>>,
    label: &'a str,
    position: Position,
) -> Element<'a, PreferenceMessage> {
    tooltip(
        content,
        container(text(label).size(12))
            .padding(6)
            .style(container::rounded_box),
        position,
    )
    .gap(8)
    .delay(TOOLTIP_DELAY)
    .snap_within_viewport(true)
    .into()
}

fn section<'a>(
    label: &'a str,
    on_reset: Option<(&'a str, PreferenceMessage)>,
    theme: &Theme,
) -> Element<'a, PreferenceMessage> {
    let text_color = theme.extended_palette().background.base.text;
    let mut header = row![
        text(label)
            .size(11)
            .font(Font {
                weight: Weight::Semibold,
                ..Font::DEFAULT
            })
            .color(text_color),
        Space::new().width(Length::Fill),
    ]
    .align_y(Vertical::Center);

    if let Some((tooltip_label, msg)) = on_reset {
        header = header.push(with_tooltip(
            button(text("Reset").size(11))
                .style(plain_icon_button_style)
                .on_press(msg)
                .padding([2.0, 6.0]),
            tooltip_label,
            Position::Top,
        ));
    }

    column![header, rule::horizontal(1).style(pref_section_rule_style)]
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

fn divider<'a>() -> Element<'a, PreferenceMessage> {
    container(Space::new().height(RULE_HEIGHT))
        .width(Length::Fill)
        .style(panel_divider_style)
        .into()
}

fn settings_list<'a>(rows: Vec<Element<'a, PreferenceMessage>>) -> Element<'a, PreferenceMessage> {
    let n = rows.len();
    let mut col = column![].spacing(PAD * 2.0).width(Length::Fill);
    for (i, r) in rows.into_iter().enumerate() {
        col = col.push(r);
        if i + 1 < n {
            col = col.push(divider());
        }
    }
    col.into()
}

fn nav_button<'a>(
    label: &'a str,
    target: PrefSection,
    active: bool,
) -> Element<'a, PreferenceMessage> {
    button(text(label).size(13))
        .width(Length::Fill)
        .padding([6.0, 8.0])
        .style(pref_nav_button_style(active))
        .on_press(PreferenceMessage::SelectSection(target))
        .into()
}

fn bar<'a>(
    content: impl Into<Element<'a, PreferenceMessage>>,
    divider_on_top: bool,
) -> Element<'a, PreferenceMessage> {
    let body = container(content)
        .width(Length::Fill)
        .height(Length::Fixed(BAR_HEIGHT))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .padding([0.0, PAD]);
    let stack = if divider_on_top {
        column![divider(), body]
    } else {
        column![body, divider()]
    }
    .width(Length::Fill);
    container(stack).width(Length::Fill).style(bar_style).into()
}

fn appearance_pane<'a>(pending: &'a Config, theme: &Theme) -> Element<'a, PreferenceMessage> {
    let rows = vec![
        setting(
            "Theme",
            "Color scheme for the application",
            ThemePicker::new(pending.theme.clone(), PreferenceMessage::SetTheme).into(),
            theme,
        ),
        setting(
            "Rounded corners",
            "Use rounded corners on UI elements",
            toggler(pending.rounded)
                .on_toggle(PreferenceMessage::SetRounded)
                .into(),
            theme,
        ),
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
    ];
    column![
        section(
            "Appearance",
            Some((
                "Reset appearance to defaults",
                PreferenceMessage::ResetAppearance
            )),
            theme,
        ),
        settings_list(rows),
    ]
    .spacing(PAD * 2.0)
    .width(Length::Fill)
    .into()
}

fn library_pane<'a>(theme: &Theme, confirming_clear: bool) -> Element<'a, PreferenceMessage> {
    let clear_control: Element<'a, PreferenceMessage> = if confirming_clear {
        row![
            button(text("Cancel").size(12))
                .on_press(PreferenceMessage::CancelClearLibrary)
                .padding([4.0, 8.0]),
            button(text("Confirm").size(12))
                .on_press(PreferenceMessage::ConfirmClearLibrary)
                .padding([4.0, 8.0])
                .style(button::danger),
        ]
        .spacing(PAD)
        .into()
    } else {
        button(text("Clear Database").size(12))
            .on_press(PreferenceMessage::ClearLibrary)
            .padding([4.0, 8.0])
            .style(button::danger)
            .into()
    };

    let rows = vec![
        setting(
            "Music library folder",
            "The folder scanned for your music collection",
            button(text("Set Folder").size(12))
                .on_press(PreferenceMessage::SetLibrary)
                .padding([4.0, 8.0])
                .into(),
            theme,
        ),
        setting(
            "Clear library",
            "Remove all tracks, albums, and playlists from the database",
            clear_control,
            theme,
        ),
    ];
    column![section("Library", None, theme), settings_list(rows)]
        .spacing(PAD * 2.0)
        .width(Length::Fill)
        .into()
}

pub fn view<'a>(
    pending: &'a Config,
    theme: &Theme,
    section_active: PrefSection,
    confirming_clear: bool,
) -> Element<'a, PreferenceMessage> {
    let header = bar(text("Preferences").size(16), false);

    let sidebar = container(
        column![
            nav_button(
                "Appearance",
                PrefSection::Appearance,
                section_active == PrefSection::Appearance,
            ),
            nav_button(
                "Library",
                PrefSection::Library,
                section_active == PrefSection::Library,
            ),
        ]
        .spacing(PAD)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fixed(PREF_SIDEBAR_WIDTH))
    .height(Length::Fill)
    .padding(PAD * 2.0);

    let pane = match section_active {
        PrefSection::Appearance => appearance_pane(pending, theme),
        PrefSection::Library => library_pane(theme, confirming_clear),
    };

    let content = scrollable(
        container(pane)
            .max_width(PREF_CONTENT_MAX_WIDTH)
            .width(Length::Fill)
            .padding(PAD * 3.0),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .direction(Direction::Vertical(
        Scrollbar::new().width(4).scroller_width(4),
    ));

    let footer = bar(
        row![
            with_tooltip(
                button(text("Reset all").size(12))
                    .style(plain_icon_button_style)
                    .on_press(PreferenceMessage::Reset)
                    .padding([4.0, 8.0]),
                "Reset all settings to defaults",
                Position::Top,
            ),
            Space::new().width(Length::Fill),
            with_tooltip(
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
                "Save",
                Position::Top,
            ),
            with_tooltip(
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
                "Cancel",
                Position::Top,
            ),
        ]
        .width(Length::Fill)
        .align_y(Vertical::Center)
        .spacing(PAD),
        true,
    );

    column![
        header,
        row![sidebar, rule::vertical(1), content]
            .width(Length::Fill)
            .height(Length::Fill),
        footer,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
