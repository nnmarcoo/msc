//! Shared widget styling: theme-derived colours and the global corner radius.
//!
//! `radius()` is the "rounded corners" setting, and every surface with corners
//! to soften reads it rather than naming its own number, so the preference
//! reaches all of them at once. Two kinds of curve deliberately do not: a shape
//! that *is* round — the scrubber head, the volume head, a toggler — stays round
//! when the setting is off, because squaring it would not be a flatter corner
//! but a different object; and a seam, hairline, or full-width bar has no
//! corners to round in the first place. `floating_bar_style` exists for that
//! reason: the same colours as `bar_style`, but for the edit-mode chip, which
//! floats over a pane and so has corners, where the preferences bars span the
//! window and must not.
//!
//! Stock iced widgets need their own bridge, since their defaults hardcode a
//! radius — `pick_list` at 2.0 — and would otherwise ignore the setting while
//! everything around them obeyed it. The wrappers here take iced's default
//! style and overwrite only the radius, so themes keep their own colours.
//!
//! The three divider styles carry meaning rather than decoration, so they are
//! kept visually distinct. A seam drags the split's ratio when free and rewrites
//! an adjacent pane's pixel lock when pinned; both do something, so both get a
//! live colour, and pinned takes its own rather than a shade of free. Only the
//! inert style, a seam that refuses drags outright, is muted, so that dulling
//! reliably reads as "nothing will happen here".

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use iced::{
    Background, Border, Theme,
    widget::{button, container, svg},
};

pub const PAD: f32 = 5.0;
pub const RADIUS: f32 = 6.0;

pub const LABEL_FONT_SIZE: f32 = 11.0;

pub const TOOLTIP_DELAY: Duration = Duration::from_millis(400);

pub const BAR_HEIGHT: f32 = 40.0;
pub const RULE_HEIGHT: f32 = 2.0;

pub const PREF_SIDEBAR_WIDTH: f32 = 160.0;
pub const PREF_CONTENT_MAX_WIDTH: f32 = 600.0;

static ACTIVE_RADIUS: OnceLock<AtomicU32> = OnceLock::new();

pub fn set_radius(rounded: bool) {
    let value = if rounded { RADIUS } else { 0.0 };
    ACTIVE_RADIUS
        .get_or_init(|| AtomicU32::new(value.to_bits()))
        .store(value.to_bits(), Ordering::Relaxed);
}

pub fn radius() -> f32 {
    f32::from_bits(
        ACTIVE_RADIUS
            .get_or_init(|| AtomicU32::new(RADIUS.to_bits()))
            .load(Ordering::Relaxed),
    )
}

pub fn bar_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.background.base.text),
        background: Some(Background::Color(palette.background.strong.color)),
        ..Default::default()
    }
}

pub fn floating_bar_style(theme: &Theme) -> container::Style {
    container::Style {
        border: iced::border::rounded(radius()),
        ..bar_style(theme)
    }
}

pub fn divider_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.primary.base.color)),
        ..Default::default()
    }
}

pub fn divider_pinned_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.success.base.color)),
        ..Default::default()
    }
}

pub fn divider_inert_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(
            palette.background.strong.color.scale_alpha(0.6),
        )),
        ..Default::default()
    }
}

pub fn tile_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);

    button::Style {
        background: lit.then_some(Background::Color(palette.background.weak.color)),
        text_color: palette.background.base.text,
        border: iced::border::rounded(radius()),
        ..button::Style::default()
    }
}

pub fn artwork_placeholder_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(
            palette.background.strong.color.scale_alpha(0.4),
        )),
        border: iced::border::rounded(radius()),
        ..Default::default()
    }
}

pub fn drop_highlight_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(
            palette.primary.base.color.scale_alpha(0.35),
        )),
        border: iced::border::rounded(radius()),
        ..Default::default()
    }
}

pub fn tooltip_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.background.base.text),
        background: Some(Background::Color(palette.background.weak.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: radius().into(),
        },
        ..Default::default()
    }
}

pub fn menu_container_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.background.base.text),
        background: Some(Background::Color(palette.background.weak.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: radius().into(),
        },
        ..Default::default()
    }
}

pub fn menu_item_hover_color(theme: &Theme) -> iced::Color {
    theme.extended_palette().background.strong.color
}

pub fn icon_button_style_container(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.background.base.text),
        border: iced::border::rounded(radius()),
        ..Default::default()
    }
}

pub fn svg_style(theme: &Theme, status: svg::Status) -> svg::Style {
    let base = theme.extended_palette().background.base.text;
    let color = match status {
        svg::Status::Hovered => base,
        svg::Status::Idle => base.scale_alpha(0.7),
    };
    svg::Style { color: Some(color) }
}

pub fn muted_text(theme: &Theme) -> iced::Color {
    theme
        .extended_palette()
        .background
        .base
        .text
        .scale_alpha(0.5)
}

pub fn pref_nav_button_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status: button::Status| {
        let palette = theme.extended_palette();
        let background = if active {
            Some(Background::Color(palette.background.strong.color))
        } else {
            match status {
                button::Status::Hovered => Some(Background::Color(palette.background.weak.color)),
                button::Status::Pressed => Some(Background::Color(palette.background.strong.color)),
                _ => None,
            }
        };

        button::Style {
            background,
            text_color: if active {
                palette.background.base.text
            } else {
                palette.background.base.text.scale_alpha(0.7)
            },
            border: iced::border::rounded(radius()),
            ..Default::default()
        }
    }
}

pub fn pick_list_style(
    theme: &Theme,
    status: iced::widget::pick_list::Status,
) -> iced::widget::pick_list::Style {
    let base = iced::widget::pick_list::default(theme, status);
    iced::widget::pick_list::Style {
        border: Border {
            radius: radius().into(),
            ..base.border
        },
        ..base
    }
}

pub fn pick_list_menu_style(theme: &Theme) -> iced::overlay::menu::Style {
    let base = iced::overlay::menu::default(theme);
    iced::overlay::menu::Style {
        border: Border {
            radius: radius().into(),
            ..base.border
        },
        ..base
    }
}

pub fn pref_rule_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            theme.extended_palette().primary.base.color,
        )),
        ..Default::default()
    }
}

pub fn pref_divider_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(
            palette.background.base.text.scale_alpha(0.15),
        )),
        ..Default::default()
    }
}

pub fn icon_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            Some(Background::Color(palette.background.weak.color))
        }
        _ => None,
    };
    button::Style {
        background,
        text_color: palette.background.base.text,
        border: iced::border::rounded(radius()),
        ..Default::default()
    }
}
