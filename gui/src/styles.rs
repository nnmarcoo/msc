//! Shared widget styling: theme-derived colours and the global corner radius.
//!
//! The three divider styles carry meaning rather than decoration, so they are
//! kept visually distinct. A seam drags the split's ratio when free and rewrites
//! an adjacent pane's pixel lock when pinned; both do something, so both get a
//! live colour, and pinned takes its own rather than a shade of free. Only the
//! inert style — a seam that refuses drags outright — is muted, so that dulling
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
