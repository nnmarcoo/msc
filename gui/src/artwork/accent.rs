//! The one accent color a pane draws with, resolved from a theme and a cover.
//!
//! Three panes tint parts of themselves — the timeline's played rail and head,
//! the volume's filled rail and head, the track info pane's title — and each can
//! be set to follow the theme or the playing record. The rule for turning a
//! sleeve into a color a pane can actually draw with is the same for all of
//! them, so it lives here rather than three times over.
//!
//! What a cover contributes is its hue and a saturation floor, never its
//! lightness. [`crate::artwork::palette`] fits the color it names to sit
//! *behind* text, capping luminance near 0.09, so drawing with it directly puts
//! a rail barely above a dark theme's background and far below a light one's.
//! Taking the lightness from the theme's own primary instead means a tinted rail
//! sits exactly where the untinted one did and reads the same on every theme —
//! the pane changes hue, not weight.
//!
//! [`tinted`] is that rule alone, and [`crate::widgets::spectrum`] reaches it
//! directly rather than through [`Accent::resolve`], because a spectrum tints
//! two colors — the quiet and loud ends of its ramp — where a rail or a heading
//! tints one, and it answers a wider question than [`Accent`] can pose. What the
//! two must not do is answer "what color is this record" differently: a layout
//! showing a visualizer beside a timeline would put both on screen at once, and
//! a saturation floor that had drifted between them would be plainly visible.
//! One function is what keeps them honest.
//!
//! Below [`MIN_SATURATION`] a sleeve is gray, its hue is whatever rounding
//! produced, and tinting to it would be arbitrary rather than characteristic. A
//! gray sleeve, a track with no art, and a stopped player all fall back to the
//! reference color untouched, so a pane always draws something deliberate.
//!
//! [`Accent::resolve`] takes the reference color rather than reading the palette
//! itself, because the panes do not all tint the same entry: a rail is drawn in
//! `primary.base` and a heading in `background.base.text`. Passing the color in
//! keeps this about the sleeve and leaves each pane owning what it would have
//! drawn anyway, which is also what makes `Theme` a pass-through rather than a
//! second lookup that could drift from the pane's own.

use iced::Color;

use crate::pane::settings::Accent;

use super::palette;

const MIN_SATURATION: f32 = 0.08;

const SATURATION: f32 = 0.55;

impl Accent {
    pub fn resolve(self, reference: Color, cover: Option<[u8; 3]>) -> Color {
        match self {
            Self::Theme => reference,
            Self::Artwork => cover
                .and_then(|cover| tinted(reference, cover))
                .unwrap_or(reference),
        }
    }
}

pub fn tinted(reference: Color, cover: [u8; 3]) -> Option<Color> {
    let (hue, saturation, _) = palette::to_hsl(cover);

    if saturation < MIN_SATURATION {
        return None;
    }

    let (_, _, lightness) = palette::to_hsl(to_bytes(reference));
    let [r, g, b] = palette::from_hsl(hue, saturation.max(SATURATION), lightness);

    Some(Color {
        a: reference.a,
        ..Color::from_rgb8(r, g, b)
    })
}

fn to_bytes(color: Color) -> [u8; 3] {
    [color.r, color.g, color.b].map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Theme;

    const RED_SLEEVE: [u8; 3] = [150, 40, 40];
    const BLUE_SLEEVE: [u8; 3] = [40, 50, 160];

    fn primary(theme: &Theme) -> Color {
        theme.extended_palette().primary.base.color
    }

    fn lightness(color: Color) -> f32 {
        palette::to_hsl(to_bytes(color)).2
    }

    #[test]
    fn following_the_theme_draws_exactly_what_the_pane_would_have() {
        for theme in [Theme::Light, Theme::Dark] {
            let reference = primary(&theme);
            assert_eq!(
                Accent::Theme.resolve(reference, Some(RED_SLEEVE)),
                reference,
                "a pane set to follow the theme was tinted by the cover anyway"
            );
        }
    }

    #[test]
    fn following_the_artwork_takes_the_hue_of_the_cover() {
        let reference = primary(&Theme::Dark);
        let red = Accent::Artwork.resolve(reference, Some(RED_SLEEVE));
        let blue = Accent::Artwork.resolve(reference, Some(BLUE_SLEEVE));

        assert!(red.r > red.b, "a red sleeve did not give a red accent");
        assert!(blue.b > blue.r, "a blue sleeve did not give a blue accent");
    }

    #[test]
    fn a_tinted_accent_keeps_the_depth_the_theme_chose() {
        for theme in crate::config::ALL_THEMES {
            for sleeve in [RED_SLEEVE, BLUE_SLEEVE] {
                let reference = primary(theme);
                let tinted = Accent::Artwork.resolve(reference, Some(sleeve));

                assert!(
                    (lightness(tinted) - lightness(reference)).abs() < 0.05,
                    "{theme} drew a tinted accent at a different depth than its own \
                     primary, so one of the two is wrong against the background"
                );
            }
        }
    }

    #[test]
    fn a_cover_with_no_hue_falls_back_to_the_theme() {
        let reference = primary(&Theme::Dark);
        for gray in [[20u8, 20, 20], [128, 128, 128], [240, 240, 240]] {
            assert_eq!(
                Accent::Artwork.resolve(reference, Some(gray)),
                reference,
                "a gray sleeve tinted the accent to an arbitrary hue"
            );
        }
    }

    #[test]
    fn nothing_playing_falls_back_to_the_theme() {
        for theme in [Theme::Light, Theme::Dark] {
            let reference = primary(&theme);
            assert_eq!(Accent::Artwork.resolve(reference, None), reference);
        }
    }

    #[test]
    fn a_faded_reference_keeps_its_alpha() {
        let faded = primary(&Theme::Dark).scale_alpha(0.7);
        let tinted = Accent::Artwork.resolve(faded, Some(RED_SLEEVE));

        assert!(
            (tinted.a - faded.a).abs() < f32::EPSILON,
            "tinting a faded color made it {} rather than {}",
            tinted.a,
            faded.a
        );
    }

    #[test]
    fn every_theme_gives_a_usable_accent_for_every_sleeve() {
        for theme in crate::config::ALL_THEMES {
            for sleeve in [RED_SLEEVE, BLUE_SLEEVE, [128, 128, 128]] {
                let accent = Accent::Artwork.resolve(primary(theme), Some(sleeve));
                assert!(
                    accent.r.is_finite() && accent.g.is_finite() && accent.b.is_finite(),
                    "{theme} gave a nonsense accent for {sleeve:?}"
                );
            }
        }
    }
}
