//! Shared widget styling: theme-derived colors and the global corner radius.
//!
//! `radius()` is the "rounded corners" setting, and every surface with corners
//! to soften reads it rather than naming its own number, so the preference
//! reaches all of them at once. Two kinds of curve deliberately do not: a shape
//! that *is* round — the scrubber head, the volume head, a toggler — stays round
//! when the setting is off, because squaring it would not be a flatter corner
//! but a different object; and a seam, hairline, or full-width bar has no
//! corners to round in the first place. `floating_bar_style` exists for that
//! reason: the same colors as `bar_style`, but for the edit-mode chip, which
//! floats over a pane and so has corners, where the preferences bars span the
//! window and must not.
//!
//! Stock iced widgets need their own bridge, since their defaults hardcode a
//! radius — `pick_list` at 2.0 — and would otherwise ignore the setting while
//! everything around them obeyed it. The wrappers here take iced's default
//! style and overwrite only the radius, so themes keep their own colors.
//!
//! The three divider styles carry meaning rather than decoration, so they are
//! kept visually distinct. A seam drags the split's ratio when free and rewrites
//! an adjacent pane's pixel lock when pinned; both do something, so both get a
//! live color, and pinned takes its own rather than a shade of free. Only the
//! inert style, a seam that refuses drags outright, is muted, so that dulling
//! reliably reads as "nothing will happen here".
//!
//! `scrim_style` is black at a low alpha rather than a palette color, because
//! its job is to take light out of whatever happens to be behind it; tinting it
//! with the theme would make it read as a surface of its own rather than as the
//! window receding. `modal_style` then takes the *base* background rather than
//! the weak one, so the dialog reads as nearer the viewer than the panes dimmed
//! behind it, and `modal_control_style` is the themed counterpart to
//! `panel_button_style`, which is white only because it sits on artwork the
//! theme does not own.
//!
//! Text sits at three weights against the theme's own background — `plain_text`
//! at full strength, `dim_text` for a label beside what it names, `faint_text`
//! for a number or duration that should be found only when looked for. They are
//! here rather than per pane because the weights are the same everywhere and
//! three panes had each written the same closure out; drifting alphas between
//! them read as one pane's labels being subtly wrong rather than as a choice.
//! `over_tint_text` and `over_tint_dim_text` are the pair for surfaces the theme
//! does *not* own — a panel tinted by cover art — where white is the only
//! reliably legible color, for the reason [`over_art_svg_style`] gives.
//!
//! `accent_heading` is the title color for a pane following the playing record.
//! The two panes that accent a title, [`crate::pane::timeline`] and
//! [`crate::pane::track_info`], share it rather than each writing the match,
//! because getting it wrong is silent: an accent keeps its reference's lightness
//! and takes only the hue, so tinting against the *text* color — near white on a
//! dark theme — gives a white title with a cast nobody would call tinted.
//! Resolving against `primary.base` instead gives the title the same color the
//! timeline's rail takes, so a pane accents as one thing.
//!
//! It is the one accent surface that cannot go through [`Accent::resolve`],
//! because that falls back to the reference it tints — right for a rail, which
//! is drawn in `primary.base` either way, and wrong for a title, which is drawn
//! in the text color and would otherwise turn the theme's accent blue the moment
//! a track had no cover to read. A title that has nothing to tint by is a title
//! nobody has tinted, so it falls back to the plain heading color, which is what
//! the pane would have drawn had the setting never been touched.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use iced::{
    Background, Border, Theme,
    widget::{button, container, svg, text},
};

use crate::pane::settings::Accent;

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

pub fn tile_style(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        ..button::Style::default()
    }
}

pub fn over_art_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Gradient(iced::Gradient::Linear(
            iced::gradient::Linear::new(iced::Radians(std::f32::consts::PI))
                .add_stop(0.0, iced::Color::TRANSPARENT)
                .add_stop(0.55, iced::Color::BLACK.scale_alpha(0.45))
                .add_stop(1.0, iced::Color::BLACK.scale_alpha(0.85)),
        ))),
        ..Default::default()
    }
}

pub fn over_art_svg_style(_theme: &Theme, status: svg::Status) -> svg::Style {
    let color = match status {
        svg::Status::Hovered => iced::Color::WHITE,
        svg::Status::Idle => iced::Color::WHITE.scale_alpha(0.8),
    };
    svg::Style { color: Some(color) }
}

pub fn over_art_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => Some(Background::Color(iced::Color::WHITE.scale_alpha(0.18))),
        button::Status::Pressed => Some(Background::Color(iced::Color::WHITE.scale_alpha(0.28))),
        _ => None,
    };

    button::Style {
        background,
        text_color: iced::Color::WHITE,
        border: iced::border::rounded(radius()),
        ..button::Style::default()
    }
}

pub fn panel_style(tint: Option<[u8; 3]>) -> impl Fn(&Theme) -> container::Style {
    move |theme: &Theme| {
        let palette = theme.extended_palette();
        let plain = palette.background.weak.color;

        let background = match tint {
            Some([r, g, b]) => Background::Gradient(iced::Gradient::Linear(
                iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
                    .add_stop(0.0, iced::Color::from_rgb8(r, g, b))
                    .add_stop(1.0, plain),
            )),
            None => Background::Color(plain),
        };

        container::Style {
            text_color: Some(palette.background.base.text),
            background: Some(background),
            border: Border {
                color: palette.background.strong.color,
                width: 1.0,
                radius: radius().into(),
            },
            ..Default::default()
        }
    }
}

pub fn panel_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => Some(Background::Color(iced::Color::WHITE.scale_alpha(0.15))),
        button::Status::Pressed => Some(Background::Color(iced::Color::WHITE.scale_alpha(0.25))),
        _ => None,
    };

    button::Style {
        background,
        text_color: iced::Color::WHITE,
        border: iced::border::rounded(radius()),
        ..Default::default()
    }
}

pub fn listing_row_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered => Some(Background::Color(
            palette.background.strong.color.scale_alpha(0.5),
        )),
        button::Status::Pressed => Some(Background::Color(palette.primary.weak.color)),
        _ => None,
    };

    button::Style {
        background,
        text_color: palette.background.base.text,
        border: iced::border::rounded(radius()),
        ..button::Style::default()
    }
}

pub fn cover_placeholder_style(theme: &Theme) -> container::Style {
    container::Style {
        border: Border::default(),
        ..artwork_placeholder_style(theme)
    }
}

pub fn plain_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().background.base.text),
    }
}

pub fn dim_text(theme: &Theme) -> text::Style {
    faded_text(theme, 0.6)
}

pub fn faint_text(theme: &Theme) -> text::Style {
    faded_text(theme, 0.4)
}

fn faded_text(theme: &Theme, alpha: f32) -> text::Style {
    text::Style {
        color: Some(
            theme
                .extended_palette()
                .background
                .base
                .text
                .scale_alpha(alpha),
        ),
    }
}

pub fn accent_heading(theme: &Theme, accent: Accent, cover: Option<[u8; 3]>) -> iced::Color {
    let palette = theme.extended_palette();
    let plain = palette.background.base.text;

    match accent {
        Accent::Theme => plain,
        Accent::Artwork => cover
            .and_then(|cover| crate::artwork::accent::tinted(palette.primary.base.color, cover))
            .unwrap_or(plain),
    }
}

pub fn over_tint_text(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(iced::Color::WHITE),
    }
}

pub fn over_tint_dim_text(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(iced::Color::WHITE.scale_alpha(0.75)),
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

pub fn scrim_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(iced::Color::BLACK.scale_alpha(0.45))),
        ..Default::default()
    }
}

pub fn modal_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.background.base.text),
        background: Some(Background::Color(palette.background.base.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: radius().into(),
        },
        ..Default::default()
    }
}

pub fn modal_control_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => palette.background.strong.color,
        _ => palette.background.weak.color,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.background.base.text,
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

pub fn key_chip_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => palette.background.strong.color,
        _ => palette.background.weak.color,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.background.base.text,
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: radius().into(),
        },
        ..Default::default()
    }
}

pub fn capturing_chip_style(theme: &Theme, _status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    button::Style {
        background: Some(Background::Color(
            palette.primary.weak.color.scale_alpha(0.3),
        )),
        text_color: palette.background.base.text,
        border: Border {
            color: palette.primary.base.color,
            width: 1.0,
            radius: radius().into(),
        },
        ..Default::default()
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

#[cfg(test)]
mod tests {
    use super::*;

    const RED_SLEEVE: [u8; 3] = [150, 40, 40];

    fn plain(theme: &Theme) -> iced::Color {
        theme.extended_palette().background.base.text
    }

    #[test]
    fn a_heading_following_the_theme_is_the_plain_title_color() {
        for theme in crate::config::ALL_THEMES {
            assert_eq!(
                accent_heading(theme, Accent::Theme, Some(RED_SLEEVE)),
                plain(theme),
                "{theme} tinted a title set to follow the theme"
            );
        }
    }

    #[test]
    fn a_heading_with_nothing_to_tint_by_stays_the_plain_title_color() {
        for theme in crate::config::ALL_THEMES {
            assert_eq!(
                accent_heading(theme, Accent::Artwork, None),
                plain(theme),
                "{theme} drew a title with no cover in its accent color rather than \
                 the color the title would have had"
            );

            for gray in [[20u8, 20, 20], [128, 128, 128], [240, 240, 240]] {
                assert_eq!(
                    accent_heading(theme, Accent::Artwork, Some(gray)),
                    plain(theme),
                    "{theme} drew a title under a gray sleeve in its accent color"
                );
            }
        }
    }

    #[test]
    fn a_heading_following_the_artwork_takes_the_record_s_color() {
        let theme = Theme::Dark;
        let tinted = accent_heading(&theme, Accent::Artwork, Some(RED_SLEEVE));

        assert!(tinted.r > tinted.b, "a red sleeve did not reach the title");
        assert_ne!(
            tinted,
            plain(&theme),
            "a title following the artwork drew the untinted color"
        );
    }

    #[test]
    fn a_tinted_heading_is_the_color_the_rail_beside_it_takes() {
        for theme in crate::config::ALL_THEMES {
            let primary = theme.extended_palette().primary.base.color;

            assert_eq!(
                accent_heading(theme, Accent::Artwork, Some(RED_SLEEVE)),
                Accent::Artwork.resolve(primary, Some(RED_SLEEVE)),
                "{theme} draws its title and its rail in different colors, so a pane \
                 following one record accents as two things"
            );
        }
    }
}
