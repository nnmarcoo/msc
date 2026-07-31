//! The color a cover is mostly made of.
//!
//! Extracted so a surface showing an album can be tinted by it, which ties the
//! panel to the record it belongs to more directly than any label: the eye reads
//! "this is that sleeve" before it reads the title. See
//! [`crate::pane::collections`].
//!
//! Taken from a 64x64 reduction rather than the full image, since the answer is
//! one color and a cover has millions of pixels; at that size the count is
//! four thousand comparisons and the result is indistinguishable. The reduction
//! is from the master the decoder already holds, so this costs no extra read.
//!
//! Colors are bucketed 16 values to a channel before counting. Photographic art
//! has almost no exactly repeated pixels — a gradient across a sleeve is
//! thousands of neighbouring shades — so counting exact values returns whichever
//! single shade happened to occur twice, which is noise. Bucketing collapses a
//! region of color into one bin, and the most populous bin is the color a
//! viewer would name.
//!
//! Pixels too grey, too dark, or too bright are not counted. A sleeve that is
//! mostly white or mostly black would otherwise return white or black, which
//! says nothing about the record and tints the panel to something the theme
//! already is. Skipping them means a monochrome cover finds whatever color it
//! does have, and one with no color at all falls through to [`None`] and the
//! panel keeps its own background rather than being tinted grey.
//!
//! The result is darkened before it is returned. It is drawn behind text, and
//! the most common color of a bright sleeve is bright: used as-is it left the
//! panel's own foreground unreadable on exactly the covers where extraction
//! worked best. Darkening is applied here rather than at the call site so that
//! every use gets a color already fit to sit behind words.

use image::RgbaImage;

/// How far a pixel's channels must spread before it counts as having a color.
const MIN_CHROMA: u8 = 20;

/// Pixels outside this range are ignored: near-black and near-white say nothing
/// about a sleeve and would tint the panel to what the theme already is.
const MIN_LEVEL: u8 = 10;
const MAX_LEVEL: u8 = 245;

/// Values per channel in a bucket. Sixteen collapses a gradient into a few bins
/// while keeping colors a viewer would call different apart.
const BUCKET: u8 = 16;

/// The edge the cover is reduced to before counting.
const SAMPLE: u32 = 64;

/// How much of the extracted color survives. It sits behind text, and the most
/// common color of a bright sleeve is bright.
const DARKEN: f32 = 0.55;

/// The dominant color of a cover, already darkened to sit behind text.
///
/// `None` when the image has no color worth naming — a greyscale sleeve, or one
/// so dark or so blown out that every pixel is skipped. The caller draws its own
/// background then, rather than a tint that means nothing.
pub fn dominant(image: &RgbaImage) -> Option<[u8; 3]> {
    let sample =
        image::imageops::resize(image, SAMPLE, SAMPLE, image::imageops::FilterType::Triangle);

    let mut bins = std::collections::HashMap::<(u8, u8, u8), u32>::new();

    for pixel in sample.pixels() {
        let [r, g, b, alpha] = pixel.0;
        // A transparent pixel has no color to contribute, whatever its channels
        // happen to hold underneath.
        if alpha < 128 || !worth_counting(r, g, b) {
            continue;
        }
        *bins
            .entry((r / BUCKET, g / BUCKET, b / BUCKET))
            .or_default() += 1;
    }

    let (bin, _) = bins.into_iter().max_by_key(|&(bin, count)| (count, bin))?;

    Some(darken(bin))
}

/// Whether a pixel says anything about what color the sleeve is.
fn worth_counting(r: u8, g: u8, b: u8) -> bool {
    let high = r.max(g).max(b);
    let low = r.min(g).min(b);

    high.saturating_sub(low) > MIN_CHROMA && high < MAX_LEVEL && low > MIN_LEVEL
}

/// A bucket back to a color, at the brightness text can sit on.
///
/// The bucket names a range, so its low end is scaled back up and then taken
/// down by [`DARKEN`]; the two fold into one multiply per channel.
fn darken((r, g, b): (u8, u8, u8)) -> [u8; 3] {
    let scale = f32::from(BUCKET) * DARKEN;
    let channel = |value: u8| (f32::from(value) * scale).round().clamp(0.0, 255.0) as u8;

    [channel(r), channel(g), channel(b)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn filled(color: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(SAMPLE, SAMPLE, Rgba(color))
    }

    /// A sleeve of one color returns that color, darkened. The exact value
    /// depends on the bucketing, so what is asserted is the hue surviving: the
    /// channel that dominated the source still dominates the answer.
    #[test]
    fn a_solid_cover_returns_its_own_color() {
        let [r, g, b] = dominant(&filled([200, 60, 60, 255])).expect("a color");

        assert!(
            r > g && r > b,
            "a red sleeve came back as ({r}, {g}, {b}), which is not red"
        );
    }

    #[test]
    fn the_color_is_darkened_to_sit_behind_text() {
        let bright = [240, 40, 40, 255];
        let [r, _, _] = dominant(&filled(bright)).expect("a color");

        assert!(
            r < bright[0],
            "a bright sleeve returned {r}, as bright as it came in, so text over \
             it would be unreadable"
        );
    }

    #[test]
    fn a_greyscale_cover_has_no_color_to_name() {
        for grey in [30u8, 128, 200] {
            assert!(
                dominant(&filled([grey, grey, grey, 255])).is_none(),
                "a grey sleeve at {grey} named a color, so the panel would be \
                 tinted to something the theme already is"
            );
        }
    }

    #[test]
    fn a_black_or_white_cover_names_nothing() {
        assert!(dominant(&filled([0, 0, 0, 255])).is_none());
        assert!(dominant(&filled([255, 255, 255, 255])).is_none());
    }

    #[test]
    fn a_transparent_cover_names_nothing() {
        assert!(
            dominant(&filled([200, 60, 60, 0])).is_none(),
            "fully transparent pixels contributed their color anyway"
        );
    }

    /// The reason pixels are bucketed rather than counted exactly: the majority
    /// color must win even when it is spread over neighbouring shades, which is
    /// what any photographic sleeve looks like.
    #[test]
    fn the_majority_color_wins_over_a_spread_of_shades() {
        let mut image = RgbaImage::from_pixel(SAMPLE, SAMPLE, Rgba([40, 40, 200, 255]));

        // A quarter of the sleeve in red, each pixel a slightly different shade,
        // so no single exact value repeats.
        for (index, pixel) in image.pixels_mut().enumerate().take(1024) {
            let drift = (index % 8) as u8;
            *pixel = Rgba([200 + drift, 60, 60, 255]);
        }

        let [r, _, b] = dominant(&image).expect("a color");
        assert!(
            b > r,
            "the scattered red beat the solid blue three quarters, so exact \
             counting would have picked noise"
        );
    }

    #[test]
    fn the_answer_does_not_depend_on_iteration_order() {
        let image = filled([200, 60, 60, 255]);

        let first = dominant(&image);
        for _ in 0..8 {
            assert_eq!(
                dominant(&image),
                first,
                "the same cover named two different colors across runs"
            );
        }
    }
}
