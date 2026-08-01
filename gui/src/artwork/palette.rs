//! The color a cover reads as.
//!
//! Extracted so a surface showing an album can be tinted by it, which ties the
//! panel to the record it belongs to more directly than any label: the eye reads
//! "this is that sleeve" before it reads the title. See
//! [`crate::pane::collections`].
//!
//! Taken from a 64x64 reduction rather than the full image, since the answer is
//! one color and a cover has millions of pixels; at that size the work is four
//! thousand pixels and the result is indistinguishable. The reduction is from
//! the master the decoder already holds, so this costs no extra read.
//!
//! The color wanted is not the one covering the most of the sleeve. That is
//! almost always a backdrop — a wash of gray sky, a black field, a beige paper
//! stock — and tinting a panel with it says nothing about the record. What the
//! eye names is the vivid part, even when it is a quarter of the cover: the
//! orange of a sunset, the pink of a portrait. So the sleeve is reduced to a
//! handful of candidate colors and the best of them is chosen, rather than the
//! commonest pixel being counted directly.
//!
//! Candidates come from a median cut. All the pixels start in one box, and the
//! box whose channels are spread widest is repeatedly split at its median until
//! there are [`BOXES`] of them. Splitting the widest first means the divisions
//! land where a sleeve actually holds distinct colors, so a cover of one hue
//! yields sixteen shades of it and a busy cover yields sixteen different colors.
//! Each box then averages to one candidate. Averaging inside a box is safe in a
//! way that averaging a whole region of the sleeve is not, because a box is by
//! construction narrow: its pixels are already near each other.
//!
//! The cut is done in place. Pixels live in one buffer for the life of the call
//! and a box is a pair of offsets into it, so splitting a box is
//! [`slice::select_nth_unstable_by_key`] over that region and then two regions
//! where there was one — no pixels move between owners and nothing is copied.
//! The boxes themselves are a fixed array on the stack, since there are never
//! more than [`BOXES`] of them, which leaves the whole extraction at one
//! allocation: the pixel buffer. That matters because this runs once per cover
//! on the decode path with a library's worth of covers behind it.
//!
//! Selection rather than a full sort, because the median is all that is wanted
//! and the order within each half is never read: that is linear where sorting is
//! not, and over sixteen splits of four thousand pixels the difference is most of
//! the cost of the cut.
//!
//! Candidates are scored the way Android's palette extractor scores them, which
//! is the same lineage as the extractors these tints are meant to resemble. The
//! score is a weighted sum of three terms: how near the candidate is to fully
//! saturated, how near it is to mid lightness, and how much of the sleeve it
//! covers. Lightness carries the most weight, saturation and population the rest
//! — so a strong color beats a dull one, and among equally strong colors the
//! larger wins. Candidates too washed out or too near black or white to name are
//! not scored at all.
//!
//! A sleeve with no vivid color anywhere scores nothing. Rather than give up,
//! the largest candidate with any tint at all is taken, so a muted record still
//! gets a panel of its own color. Only a sleeve with no color whatsoever — true
//! grayscale — falls through to [`None`], and the panel keeps its own background
//! rather than being tinted gray.
//!
//! The winner is finally fitted to sit behind text. What is capped is luminance,
//! not lightness: contrast depends on the former, and the two differ by a factor
//! of ten around the hue circle, so a bound on lightness that suits blue still
//! leaves yellow sitting too near white. It is a ceiling rather than a target —
//! a cover already dark enough keeps the depth it came with. Pinning every tint
//! to one depth was tried and it flattened a wall of covers to a single shade,
//! which lost exactly the variety the tinting is for. A floor underneath catches
//! the near-black, and a saturation floor catches the near-gray, so no panel
//! comes back looking like an accident.
//!
//! The ceiling is reached by bisecting lightness, since luminance has no closed
//! form in it once the hue is fixed. [`REFINEMENTS`] steps resolve finer than
//! the eight bits returned, and none of them run on a color already dark enough
//! to pass.
//!
//! The constants: [`SAMPLE`] is the edge the cover is reduced to. [`BOXES`] is
//! how many candidates the cut produces, sixteen being what Android's extractor
//! uses for the same job and enough that a busy sleeve keeps its minor colors
//! without the scoring having to sift noise. [`MIN_ALPHA`] is how opaque a pixel
//! must be before it has a color to contribute, whatever its channels hold
//! underneath. The `WEIGHT_` trio is what a candidate is scored on, lightness
//! weighing most because a color too dark or too pale is unusable whatever else
//! it has, and the `TARGET_` pair is what the scoring aims at. The `MIN_SCORED_`
//! and `MAX_SCORED_` bounds exclude what no viewer would name a sleeve by, and
//! [`MIN_FALLBACK_SATURATION`] is the lower bar the fallback settles for — below
//! it lies a gray the theme's own background says better. [`MAX_LUMINANCE`] is
//! the ceiling described above, and [`MIN_LIGHTNESS`] with [`MIN_SATURATION`]
//! are the floors that keep a near-black or near-gray sleeve from tinting a
//! panel to nothing.

use image::RgbaImage;

const SAMPLE: u32 = 64;
const BOXES: usize = 16;
const MIN_ALPHA: u8 = 128;

const WEIGHT_SATURATION: f32 = 0.24;
const WEIGHT_LIGHTNESS: f32 = 0.52;
const WEIGHT_POPULATION: f32 = 0.24;

const TARGET_SATURATION: f32 = 1.0;
const TARGET_LIGHTNESS: f32 = 0.5;

const MIN_SCORED_SATURATION: f32 = 0.35;
const MIN_SCORED_LIGHTNESS: f32 = 0.3;
const MAX_SCORED_LIGHTNESS: f32 = 0.7;

const MIN_FALLBACK_SATURATION: f32 = 0.08;

const MAX_LUMINANCE: f32 = 0.09;
const REFINEMENTS: usize = 10;

const MIN_LIGHTNESS: f32 = 0.16;
const MIN_SATURATION: f32 = 0.30;

#[derive(Clone, Copy, Default)]
struct Region {
    start: usize,
    end: usize,
}

impl Region {
    fn len(self) -> usize {
        self.end - self.start
    }

    fn range(self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

pub fn dominant(image: &RgbaImage) -> Option<[u8; 3]> {
    let sample =
        image::imageops::resize(image, SAMPLE, SAMPLE, image::imageops::FilterType::Triangle);

    let mut pixels: Vec<[u8; 3]> = Vec::with_capacity(sample.pixels().len());
    pixels.extend(
        sample
            .pixels()
            .filter(|pixel| pixel.0[3] >= MIN_ALPHA)
            .map(|pixel| [pixel.0[0], pixel.0[1], pixel.0[2]]),
    );

    if pixels.is_empty() {
        return None;
    }

    let (boxes, count) = median_cut(&mut pixels);
    let boxes = &boxes[..count];

    let largest = boxes.iter().map(|region| region.len()).max()?;

    let best = boxes
        .iter()
        .filter_map(|region| {
            let color = average(&pixels[region.range()]);
            score(color, region.len(), largest).map(|score| (color, score))
        })
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(color, _)| color);

    let chosen = best.or_else(|| {
        boxes
            .iter()
            .map(|region| (average(&pixels[region.range()]), region.len()))
            .filter(|&(color, _)| to_hsl(color).1 > MIN_FALLBACK_SATURATION)
            .max_by_key(|&(_, count)| count)
            .map(|(color, _)| color)
    })?;

    Some(fit_for_text(chosen))
}

fn median_cut(pixels: &mut [[u8; 3]]) -> ([Region; BOXES], usize) {
    let mut boxes = [Region::default(); BOXES];
    boxes[0] = Region {
        start: 0,
        end: pixels.len(),
    };
    let mut count = 1;

    while count < BOXES {
        let Some((index, channel, _)) = boxes[..count]
            .iter()
            .enumerate()
            .filter(|(_, region)| region.len() > 1)
            .map(|(index, region)| {
                let (channel, spread) = widest(&pixels[region.range()]);
                (index, channel, spread)
            })
            .max_by_key(|&(_, _, spread)| spread)
            .filter(|&(_, _, spread)| spread > 0)
        else {
            break;
        };

        let region = boxes[index];
        let middle = region.start + region.len() / 2;

        pixels[region.range()].select_nth_unstable_by_key(region.len() / 2, |pixel| pixel[channel]);

        boxes[index] = Region {
            start: region.start,
            end: middle,
        };
        boxes[count] = Region {
            start: middle,
            end: region.end,
        };
        count += 1;
    }

    (boxes, count)
}

fn widest(pixels: &[[u8; 3]]) -> (usize, u8) {
    let mut low = [u8::MAX; 3];
    let mut high = [u8::MIN; 3];

    for pixel in pixels {
        for channel in 0..3 {
            low[channel] = low[channel].min(pixel[channel]);
            high[channel] = high[channel].max(pixel[channel]);
        }
    }

    (0..3)
        .map(|channel| (channel, high[channel].saturating_sub(low[channel])))
        .max_by_key(|&(_, spread)| spread)
        .unwrap_or((0, 0))
}

fn average(pixels: &[[u8; 3]]) -> [u8; 3] {
    let mut sums = [0u32; 3];

    for pixel in pixels {
        for (sum, value) in sums.iter_mut().zip(pixel) {
            *sum += u32::from(*value);
        }
    }

    let count = pixels.len().max(1) as u32;
    sums.map(|sum| (sum / count) as u8)
}

fn score(color: [u8; 3], population: usize, largest: usize) -> Option<f32> {
    let (_, saturation, lightness) = to_hsl(color);

    if saturation < MIN_SCORED_SATURATION
        || !(MIN_SCORED_LIGHTNESS..=MAX_SCORED_LIGHTNESS).contains(&lightness)
    {
        return None;
    }

    let toward = |value: f32, target: f32| 1.0 - (value - target).abs();
    let share = population as f32 / largest.max(1) as f32;

    Some(
        WEIGHT_SATURATION * toward(saturation, TARGET_SATURATION)
            + WEIGHT_LIGHTNESS * toward(lightness, TARGET_LIGHTNESS)
            + WEIGHT_POPULATION * share,
    )
}

fn fit_for_text(color: [u8; 3]) -> [u8; 3] {
    let (hue, saturation, lightness) = to_hsl(color);

    let saturation = saturation.max(MIN_SATURATION);
    let lightness = lightness.max(MIN_LIGHTNESS);
    let fitted = from_hsl(hue, saturation, lightness);

    if luminance(fitted) <= MAX_LUMINANCE {
        return fitted;
    }

    let (mut low, mut high) = (0.0, lightness);
    for _ in 0..REFINEMENTS {
        let middle = f32::midpoint(low, high);
        if luminance(from_hsl(hue, saturation, middle)) > MAX_LUMINANCE {
            high = middle;
        } else {
            low = middle;
        }
    }

    from_hsl(hue, saturation, low)
}

fn luminance([r, g, b]: [u8; 3]) -> f32 {
    let channel = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };

    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

pub fn to_hsl([r, g, b]: [u8; 3]) -> (f32, f32, f32) {
    let (r, g, b) = (
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
    );

    let high = r.max(g).max(b);
    let low = r.min(g).min(b);
    let range = high - low;

    let lightness = f32::midpoint(high, low);

    if range <= f32::EPSILON {
        return (0.0, 0.0, lightness);
    }

    let saturation = range / (1.0 - (2.0 * lightness - 1.0).abs()).max(f32::EPSILON);

    let hue = if r >= g && r >= b {
        (g - b) / range / 6.0
    } else if g >= b {
        ((b - r) / range + 2.0) / 6.0
    } else {
        ((r - g) / range + 4.0) / 6.0
    };

    (hue.rem_euclid(1.0), saturation.clamp(0.0, 1.0), lightness)
}

pub fn from_hsl(hue: f32, saturation: f32, lightness: f32) -> [u8; 3] {
    let range = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let second = range * (1.0 - ((hue * 6.0) % 2.0 - 1.0).abs());
    let base = lightness - range / 2.0;

    let (r, g, b) = match (hue * 6.0) as u8 {
        0 => (range, second, 0.0),
        1 => (second, range, 0.0),
        2 => (0.0, range, second),
        3 => (0.0, second, range),
        4 => (second, 0.0, range),
        _ => (range, 0.0, second),
    };

    [r, g, b].map(|value| ((value + base) * 255.0).round().clamp(0.0, 255.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn filled(color: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(SAMPLE, SAMPLE, Rgba(color))
    }

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

    /// The subtitle [`crate::styles::over_tint_dim_text`] draws in white at
    /// three-quarter alpha is the dimmest thing put on a tint, so it is the case
    /// that has to clear the bar rather than the title above it.
    #[test]
    fn the_dimmest_text_keeps_its_contrast_on_every_hue() {
        for hue in 0u8..48 {
            let [r, g, b] = from_hsl(f32::from(hue) / 48.0, 0.9, 0.5);
            let tint = dominant(&filled([r, g, b, 255])).expect("a color");

            let blended =
                tint.map(|channel| (255.0 * 0.75 + f32::from(channel) * 0.25).round() as u8);
            let ratio = (luminance(blended) + 0.05) / (luminance(tint) + 0.05);

            assert!(
                ratio > 4.5,
                "the subtitle on {tint:?} is only {ratio:.1}:1, below the 4.5:1 \
                 that body text needs"
            );
        }
    }

    /// The point of the whole approach: the color a viewer would name the sleeve
    /// by wins even when a duller color covers far more of it.
    #[test]
    fn a_vivid_minority_beats_a_muted_majority() {
        let mut image = RgbaImage::from_pixel(SAMPLE, SAMPLE, Rgba([150, 150, 110, 255]));
        for pixel in image.pixels_mut().take(1024) {
            *pixel = Rgba([220, 30, 30, 255]);
        }

        let [r, g, b] = dominant(&image).expect("a color");
        assert!(
            r > g && r > b,
            "the sleeve came back as ({r}, {g}, {b}), the muted majority, rather \
             than the red a viewer would name it by"
        );
    }

    #[test]
    fn a_dark_sleeve_is_named_by_the_color_on_it() {
        let mut image = RgbaImage::from_pixel(SAMPLE, SAMPLE, Rgba([12, 12, 14, 255]));
        for pixel in image.pixels_mut().take(700) {
            *pixel = Rgba([40, 120, 220, 255]);
        }

        let [r, g, b] = dominant(&image).expect("a color");
        assert!(
            b > r && b > g,
            "a black sleeve with a blue mark came back as ({r}, {g}, {b}), so the \
             backdrop won and the panel is tinted to what the theme already is"
        );
    }

    /// Tints must not all land on one depth, or a wall of covers reads as a
    /// single color and the tinting stops telling records apart.
    #[test]
    fn different_covers_keep_different_depths() {
        let depths: Vec<f32> = [
            [40u8, 40, 120, 255],
            [230, 180, 40, 255],
            [90, 200, 90, 255],
            [200, 40, 120, 255],
        ]
        .iter()
        .map(|&cover| luminance(dominant(&filled(cover)).expect("a color")))
        .collect();

        let high = depths.iter().copied().fold(f32::MIN, f32::max);
        let low = depths.iter().copied().fold(f32::MAX, f32::min);

        assert!(
            high > low * 1.5,
            "four different covers came back within {low} to {high}, so the \
             panels no longer tell them apart"
        );
    }

    /// The reason a median cut is used rather than counting exact values:
    /// photographic art repeats almost no pixel exactly, so the majority color
    /// must win even when spread over neighboring shades.
    #[test]
    fn the_majority_color_wins_over_a_spread_of_shades() {
        let mut image = RgbaImage::from_pixel(SAMPLE, SAMPLE, Rgba([40, 40, 200, 255]));

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
    fn a_grayscale_cover_has_no_color_to_name() {
        for gray in [30u8, 128, 200] {
            assert!(
                dominant(&filled([gray, gray, gray, 255])).is_none(),
                "a gray sleeve at {gray} named a color, so the panel would be \
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

    /// A sleeve with no vivid color still gets a tint of its own, since the
    /// alternative is an untinted panel next to tinted ones.
    #[test]
    fn a_muted_sleeve_still_names_a_color() {
        let [r, g, b] = dominant(&filled([120, 96, 92, 255])).expect("a color");

        assert!(
            r > b,
            "a muted warm sleeve came back as ({r}, {g}, {b}), which is not the \
             color it is"
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

    /// A cover of one flat color cannot be cut into distinct regions, so the
    /// median cut has to stop early rather than spin or split empty ranges.
    #[test]
    fn a_cover_with_nothing_to_split_still_answers() {
        let mut pixels = vec![[80u8, 20, 20]; 64];
        let (_, count) = median_cut(&mut pixels);

        assert_eq!(
            count, 1,
            "a flat cover was cut into {count} boxes, so the split ran on a \
             region with no spread to divide"
        );
    }

    /// The cut partitions one buffer, so the boxes have to tile it exactly:
    /// every pixel in one box, none counted twice, or the populations the
    /// scoring weighs are wrong.
    #[test]
    fn the_boxes_tile_the_pixels_exactly() {
        let mut pixels: Vec<[u8; 3]> = (0..512u32)
            .map(|index| {
                let value = (index % 256) as u8;
                [value, value.wrapping_mul(3), value.wrapping_mul(7)]
            })
            .collect();
        let total = pixels.len();

        let (mut boxes, count) = median_cut(&mut pixels);
        let boxes = &mut boxes[..count];
        boxes.sort_by_key(|region| region.start);

        assert_eq!(boxes.first().map(|region| region.start), Some(0));
        assert_eq!(boxes.last().map(|region| region.end), Some(total));

        for pair in boxes.windows(2) {
            assert_eq!(
                pair[0].end, pair[1].start,
                "boxes ({}..{}) and ({}..{}) do not meet, so pixels were lost or \
                 double counted",
                pair[0].start, pair[0].end, pair[1].start, pair[1].end
            );
        }
    }
}
