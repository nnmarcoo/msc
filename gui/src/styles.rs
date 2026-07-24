use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

use iced::{
    Background, Theme,
    widget::{button, container},
};

pub const PAD: f32 = 5.0;
pub const BAR_HEIGHT: f32 = 56.0;
pub const RADIUS: f32 = 6.0;

pub const ROW_FONT_SIZE: f32 = 13.0;
pub const LABEL_FONT_SIZE: f32 = 11.0;

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

pub fn panel_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.background.base.text),
        background: Some(Background::Color(palette.background.weak.color)),
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

pub fn row_style(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let palette = theme.extended_palette();
        let background = if selected {
            Some(Background::Color(
                palette.primary.base.color.scale_alpha(0.25),
            ))
        } else {
            match status {
                button::Status::Hovered | button::Status::Pressed => {
                    Some(Background::Color(palette.background.weak.color))
                }
                _ => None,
            }
        };
        button::Style {
            background,
            text_color: palette.background.base.text,
            border: iced::border::rounded(radius()),
            ..Default::default()
        }
    }
}
