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
//! Text sits at three weights against the theme's own background — `plain_text`
//! at full strength, `dim_text` for a label beside what it names, `faint_text`
//! for a number or duration that should be found only when looked for. They are
//! here rather than per pane because the weights are the same everywhere and
//! three panes had each written the same closure out; drifting alphas between
//! them read as one pane's labels being subtly wrong rather than as a choice.
//! `over_tint_text` and `over_tint_dim_text` are the pair for surfaces the theme
//! does *not* own — a panel tinted by cover art — where white is the only
//! reliably legible color, for the reason [`over_art_svg_style`] gives.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use iced::{
    Background, Border, Theme,
    widget::{button, container, svg, text},
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

/// A cover tile: the art alone, with no chrome of its own.
///
/// Transparent rather than filled, because the tile *is* the image and a plate
/// behind it would show only as a rim wherever the art did not reach the corner.
/// Square, like the cover it wraps and the scrim over it — see
/// [`over_art_style`] for why the three agree to have no corners at all.
pub fn tile_style(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        ..button::Style::default()
    }
}

/// The scrim behind controls drawn over cover art.
///
/// A gradient from transparent at the top to black at the foot rather than a
/// flat panel, so the label sits on darkness that the art fades into instead of
/// behind a box with an edge cutting across the sleeve. It spans the whole tile
/// for that reason: a scrim only as tall as its text has a hard line above it
/// wherever the cover is pale.
///
/// Fixed dark rather than theme-derived, because what it sits on is an image
/// and not a surface the theme chose: a light theme's own weak color over a
/// dark sleeve leaves white text on white, and the reverse over a pale one. The
/// scrim is what makes the label legible whatever the cover happens to be, so it
/// answers to the art rather than to the palette, and the text and icons over it
/// are white for the same reason — see [`over_art_svg_style`].
///
/// Square, and so are the cover beneath it and the button around both. A tile is
/// several layers drawn over one another, and a rounded corner is rasterized
/// separately by each: an image rounds in its own shader against its own rect, a
/// container rounds a quad against the container's, and the arcs differ by a
/// fraction of a pixel where they meet, which reads as a soft or doubled corner.
/// Squaring all three makes that unrepresentable rather than tuned away. This is
/// the one family of surfaces that deliberately ignores [`radius`], for the same
/// kind of reason a scrubber head keeps its circle: the shape is not chrome the
/// theme owns, it is the edge of a picture.
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

/// An icon drawn over cover art: white, and brightening under the pointer.
///
/// Not [`svg_style`], which colors from the palette's text: over a black scrim
/// a light theme's near-black icon is invisible. What the icon sits on is the
/// scrim rather than the theme's background, so it answers to that.
pub fn over_art_svg_style(_theme: &Theme, status: svg::Status) -> svg::Style {
    let color = match status {
        svg::Status::Hovered => iced::Color::WHITE,
        svg::Status::Idle => iced::Color::WHITE.scale_alpha(0.8),
    };
    svg::Style { color: Some(color) }
}

/// A control over cover art: the icon alone until pressed or pointed at.
///
/// Its hover fill is white at low alpha rather than the palette's, for the same
/// reason the icon is white: a theme-colored chip over a black scrim reads as a
/// grey box stuck on the sleeve.
///
/// The control keeps its radius where the tile does not: it is a small chip well
/// inside the cover rather than a layer sharing the tile's edge, so its curve
/// meets nothing it could disagree with.
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

/// An expanded collection's panel, tinted by the cover it belongs to.
///
/// The tint fades from the color at the cover's edge into the theme's own
/// background across the panel, so the surface reads as belonging to that record
/// rather than as a colored slab: the art bleeds into the panel beside it and
/// the far end stays a surface the theme owns, which is where the track list
/// sits and where text has to stay readable.
///
/// `None` leaves the panel its plain background. A greyscale sleeve names no
/// color, and neither does one whose art has not been read yet — see
/// [`crate::artwork::Cache::color`] — so the panel is untinted until the cover
/// it is showing arrives, and then tints in the same frame the cover appears.
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

/// A control on a panel: the icon alone until pointed at.
///
/// Not [`icon_button_style`], which fills with `background.weak` on hover — the
/// same color an untinted panel is, so its feedback was invisible on exactly
/// this surface.
///
/// White at low alpha rather than a palette color, for the reason
/// [`over_art_button_style`] is: this control sits at the tinted end of the
/// panel, where the background is a color taken from the cover rather than one
/// the theme chose, so no palette entry is reliably distinct from it. A wash of
/// white lightens whatever is underneath, which reads as a highlight over any
/// tint and over the plain background alike.
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

/// A row inside a panel: nothing until the pointer is on it.
///
/// Hover and press are different fills rather than one shared "lit" state, so a
/// click reads as landing rather than merely as the pointer still being there.
/// The hover is `strong` at part alpha because the row spans the panel: a full
/// strength bar the width of the listing is louder than the thing it highlights,
/// where the same color under a small control reads as a chip.
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

/// The placeholder in a collections grid: the same fill, squared.
///
/// A separate style rather than a flag on [`artwork_placeholder_style`] because
/// the two answer to different things. The artwork pane's placeholder is a
/// surface the theme owns and rounds with everything else; this one stands in
/// for a cover and must have the same edge as the covers beside it, which are
/// square for the reason [`over_art_style`] gives. A grid where the tiles with
/// art were square and the ones without were rounded would read as a mistake.
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
