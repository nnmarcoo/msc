//! The spectrum bars: a fixed set of analyzer bins drawn at whatever density fits.
//!
//! [`verse_core`] hands over a fixed [`NUM_BINS`] bands, log-spaced and already
//! normalized by the analyzer's AGC. That count is a property of the FFT, not of
//! the pane, so the widget treats it as a ceiling and folds the bins down to the
//! number of bars the width can actually show. A pane at the layout floor gets a
//! handful of wide bars and a full-width one gets all [`NUM_BINS`]; past that the
//! bars widen rather than multiply, because interpolating beyond the bins would
//! invent detail the transform never resolved.
//!
//! That ceiling is why [`NUM_BINS`] is a question about the density setting and
//! not only about the analyzer. Every width at which two densities both ask for
//! more bars than there are bins is a width where they draw the same picture and
//! the setting does nothing; when the ceiling was 32 that was most of a normal
//! window, and the picker moved without the pane changing. Raising it moves the
//! collapse out past any pane a real layout produces, which is the only fix that
//! keeps the no-interpolation rule above.
//!
//! Folding takes the maximum of each group, not the mean. Averaging a peak
//! against its quiet neighbors pulls transients down, and the visible result is
//! a display that reads as sluggish next to the audio; the max keeps a
//! transient's height at every density, so the same music has the same shape
//! whether the pane is showing 4 bars or 32.
//!
//! Groups are contiguous and their sizes differ by at most one, so the bass end
//! never quietly gets more bins per bar than the treble end. That matters because
//! the bins are already log-spaced: weighting them unevenly a second time would
//! tilt the spectrum's shape as a function of pane width alone.
//!
//! The pitch is the width a bar wants including its gap, and the bar count falls
//! out of dividing the pane by it. Deriving the count from a target pitch rather
//! than from breakpoints is what keeps the bars a readable thickness at every
//! size, which is the failure the previous version had: it drew all 32 bins
//! always, so a narrow pane gave each bar a sub-pixel slice and a wide one gave
//! 32 slabs. The pitch itself comes from
//! [`crate::pane::settings::Density`], so the same rule serves every density:
//! a coarser setting asks for wider bars and therefore fewer of them, at every
//! pane size rather than past some breakpoint.
//!
//! The gap is a fraction of the pitch rather than a constant, so it shrinks with
//! the bars instead of eating them; at the floor a constant gap is most of the
//! bar. [`MIN_BAR`] then guarantees a bar is never thinner than a pixel, since a
//! sub-pixel quad is a rounding artifact rather than a bar.
//!
//! Below [`COMPACT_WIDTH`] the pitch rule is abandoned for a flat
//! [`COMPACT_BARS`]. A pane can be dragged down to [`crate::layout::MIN_PANE`],
//! and at that size the pitch still asks for several bars in a strip barely wide
//! enough for one: the result is a row of hairlines that flicker rather than a
//! spectrum anyone can read. Three is the fewest that still says something a
//! level meter does not — [`group`] splits the bins evenly, so they land on bass,
//! mid and treble — and at that count each block is wide enough to have a
//! readable height even in the narrowest pane the layout permits. The cutoff is
//! a floor on the count rather than a separate drawing path, so folding, color,
//! caps and the peak markers all keep working with no second implementation to
//! keep in step.
//!
//! Every bar keeps [`FLOOR`] of height at silence. A spectrum that renders
//! nothing at all is indistinguishable from a pane that has failed to draw, and
//! the resting line also gives the rounded caps something to sit on.
//!
//! The widget paints no background. The pane chrome owns that, and the previous
//! version filling its own bounds is what made it fight the rounded corners.
//! Color comes from the palette for the same reason: a hardcoded ramp toward
//! gray inverted its meaning under a light theme. Every tint interpolates
//! between two palette colors, so all three stay part of the theme; what
//! differs is only what drives the interpolation — nothing, the bar's level, or
//! its position across the row. `Flat` sits at the strong tone rather than the
//! weak one, since with no ramp to climb the weak end leaves every bar washed
//! out, and `Spectrum` divides by one less than the count, so a lone bar takes
//! the start of the ramp rather than dividing by zero.
//!
//! `Amplitude` spends the whole ramp rather than a fraction of it. Scaling the
//! interpolation down meant a bar at full scale still stopped short of the
//! strong tone, so the color the setting exists to reach was one nothing could
//! ever show. `Spectrum` runs primary to danger because those are the two the
//! palette guarantees are far apart: several themes verse ships — Nord most
//! plainly — pick an accent and a success color that are both muted greens, and
//! a ramp between them is a ramp the eye cannot read. A theme whose accent and
//! error color looked alike would be unusable well before this widget noticed.
//!
//! `Artwork` takes the hue of the playing record's cover and nothing else, by
//! the rule [`crate::artwork::accent::tinted`] states and owns: the sleeve gives
//! a hue and a saturation floor, the theme keeps the lightness. The ramp is
//! therefore `Amplitude`'s two ends with the record's hue laid over both, as
//! readable on every theme as the default tint while still recognizably the
//! record's color. A sleeve too gray to have a hue tints nothing, and a ramp
//! with one tinted end and one untinted would be a gradient into the theme, so
//! both ends fall back together and the pane draws its default ramp.
//!
//! Sharing that rule rather than restating it is deliberate: a timeline set to
//! follow the artwork can sit in the same layout as this widget, and a
//! saturation floor that had drifted between the two would be visible as one
//! pane disagreeing with the next about the same record. The color itself
//! arrives from [`crate::artwork::Cache`] already extracted — the pane looks it
//! up only when this tint is selected, and never touches an image itself.
//!
//! Which two colors a tint runs between depends on the theme, the setting and
//! the cover, and none of those change between the bars of one frame, so
//! [`Ramp`] resolves them once and each bar only walks the interpolation. Doing
//! it per bar meant re-reading the extended palette up to [`NUM_BINS`] times a
//! frame and, under `Artwork`, running two HSL round-trips per bar to reach the
//! same answer every time.
//!
//! A peak marker is the bar's own color lifted [`PEAK_LIFT`] toward the
//! palette's text. Drawing it in the text color outright made it a white line
//! on a dark theme and a black one on a light theme, which read as a piece of
//! window chrome that had strayed into the visualization rather than as part of
//! it; going the whole way to the bar's color is the opposite failure, since
//! under `Flat` every bar is already exactly that and the marker disappears.
//! Meeting partway keeps it inside the spectrum's palette while leaving it
//! plainly brighter than what it sits above, at every tint.
//!
//! Markers are suppressed while the peak is still inside its bar, where the
//! marker would read as a brighter stripe across the top rather than as a
//! marker, and [`PEAK_GAP`] holds a hairline of background under one that is
//! only just clear. Without it a marker one pixel above its bar still touched
//! it, which looks like a bar with a lighter cap rather than a mark floating
//! over one.
//!
//! [`State`] is a fixed [`NUM_BINS`] array and a count rather than a collection
//! sized to the bars in use, because the bar count is capped at that same
//! ceiling: the worst case is a few hundred bytes, known at compile time. That
//! keeps the whole marker path free of allocation, which matters most exactly
//! when it runs hardest — a divider drag changes the bar count on every frame,
//! and a growing collection would reallocate on each one. `forget` and a resize
//! both become a single write to the count, and [`State::markers`] hands the
//! renderer the live prefix so a stale marker past the current count cannot be
//! drawn.
//!
//! The markers live in the widget's own [`State`] because a marker is defined by
//! what the bars did *before* this frame, and they advance in `update` rather
//! than `draw`, which is handed that state immutably. The app already redraws
//! this pane on its animation tick whenever a track is loaded, so the markers
//! are paced by the same clock as the bars and nothing extra has to ask for a
//! frame; [`PEAK_FALL`] is therefore per frame rather than per second. A resize
//! seeds the new bars from what the audio is doing now, so widening a pane does
//! not drop a row of markers in from the floor.
//!
//! A peak holds still for [`PEAK_HANG`] frames before it starts to move, and
//! then falls at [`PEAK_FALL`] scaled by [`PEAK_ACCEL`] for every frame it has
//! been falling, rather than at a constant rate. That tick is 16ms, so the
//! previous constant of 0.03 per frame took a peak from the top to the floor in
//! about half a second — fast enough that the marker was always sitting on its
//! own bar, which is why the setting looked like it did nothing. The hang is
//! what makes a peak legible at all, since a mark that leaves the instant the
//! note does is never still long enough to be read, and the acceleration is what
//! stops the much slower rate that requires from leaving stale marks over quiet
//! passages: a marker is still near the top a second after its note and has
//! caught back down to the music within about three, while one being renewed
//! every few frames never gets past the start of its fall. Each bar keeps its
//! own rate, reset on every new peak, so a bar being driven hard and a bar that
//! has gone quiet do not share a fall speed.
//!
//! Turning the setting off clears that state rather than leaving `update` early.
//! Peaks the widget stops advancing are still peaks it remembers, so a pane
//! switched off and back on drew the markers from whenever it was switched off —
//! a row of stale marks sitting above bars that had long since moved. Clearing
//! makes coming back identical to arriving for the first time, which is what the
//! resize path already does.

use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout};
use iced::{
    Background, Border, Color, Element, Event, Length, Rectangle, Renderer, Size, Theme, mouse,
};

use verse_core::NUM_BINS;

use crate::artwork::accent;
use crate::pane::settings::{Tint, Visualizer};

const GAP_RATIO: f32 = 0.25;

const MIN_BARS: usize = 4;
const MIN_BAR: f32 = 1.0;

const COMPACT_BARS: usize = 3;
const COMPACT_WIDTH: f32 = 96.0;

const FLOOR: f32 = 2.0;

const MIN_HEIGHT: f32 = 24.0;

const PEAK_THICKNESS: f32 = 2.0;

const PEAK_HANG: u8 = 30;

const PEAK_FALL: f32 = 0.003;

const PEAK_ACCEL: f32 = 1.02;

const PEAK_GAP: f32 = 1.0;

const PEAK_LIFT: f32 = 0.45;

pub struct Spectrum {
    bins: [f32; NUM_BINS],
    settings: Visualizer,
    cover: Option<[u8; 3]>,
}

impl Spectrum {
    pub fn new(bins: [f32; NUM_BINS], settings: Visualizer, cover: Option<[u8; 3]>) -> Self {
        Self {
            bins,
            settings,
            cover,
        }
    }

    pub const fn min_height() -> f32 {
        MIN_HEIGHT
    }
}

#[derive(Clone, Copy)]
struct Peak {
    level: f32,
    hang: u8,
    fall: f32,
}

impl Peak {
    const fn held(level: f32) -> Self {
        Self {
            level,
            hang: PEAK_HANG,
            fall: PEAK_FALL,
        }
    }

    fn advance(&mut self, level: f32) {
        if level >= self.level {
            *self = Self::held(level);
        } else if self.hang > 0 {
            self.hang -= 1;
        } else {
            self.level = (self.level - self.fall).max(level);
            self.fall *= PEAK_ACCEL;
        }
    }
}

struct State {
    peaks: [Peak; NUM_BINS],
    bars: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            peaks: [Peak::held(0.0); NUM_BINS],
            bars: 0,
        }
    }
}

impl State {
    fn forget(&mut self) {
        self.bars = 0;
    }

    fn markers(&self) -> &[Peak] {
        &self.peaks[..self.bars]
    }

    fn advance(&mut self, bins: &[f32; NUM_BINS], bars: usize) {
        let bars = bars.min(NUM_BINS);
        let seeding = self.bars != bars;
        self.bars = bars;

        for (index, peak) in self.peaks[..bars].iter_mut().enumerate() {
            let level = amplitude(bins, index, bars);
            if seeding {
                *peak = Peak::held(level);
            } else {
                peak.advance(level);
            }
        }
    }
}

fn bar_count(width: f32, pitch: f32) -> usize {
    if !width.is_finite() || width < COMPACT_WIDTH || pitch <= 0.0 {
        return COMPACT_BARS;
    }

    ((width / pitch) as usize).clamp(MIN_BARS, NUM_BINS)
}

fn group(index: usize, bars: usize) -> (usize, usize) {
    let bars = bars.max(1);
    let start = index * NUM_BINS / bars;
    let end = (index + 1) * NUM_BINS / bars;
    (start.min(NUM_BINS), end.clamp(start + 1, NUM_BINS))
}

fn amplitude(bins: &[f32; NUM_BINS], index: usize, bars: usize) -> f32 {
    let (start, end) = group(index, bars);
    bins[start..end]
        .iter()
        .copied()
        .fold(0.0f32, f32::max)
        .clamp(0.0, 1.0)
}

fn bar_rect(bounds: Rectangle, index: usize, bars: usize, amplitude: f32) -> Rectangle {
    let pitch = bounds.width / bars as f32;
    let gap = pitch * GAP_RATIO;
    let width = (pitch - gap).max(MIN_BAR);

    let height = (amplitude * bounds.height).max(FLOOR).min(bounds.height);

    Rectangle {
        x: bounds.x + index as f32 * pitch + gap / 2.0,
        y: bounds.y + bounds.height - height,
        width,
        height,
    }
}

#[derive(Clone, Copy)]
struct Ramp {
    quiet: Color,
    loud: Color,
}

impl Ramp {
    fn new(theme: &Theme, tint: Tint, cover: Option<[u8; 3]>) -> Self {
        let palette = theme.extended_palette();
        let (quiet, loud) = match tint {
            Tint::Flat => (palette.primary.strong.color, palette.primary.strong.color),
            Tint::Amplitude => (palette.primary.weak.color, palette.primary.strong.color),
            Tint::Spectrum => (palette.primary.strong.color, palette.danger.strong.color),
            Tint::Artwork => cover
                .and_then(|cover| Self::from_cover(theme, cover))
                .unwrap_or((palette.primary.weak.color, palette.primary.strong.color)),
        };

        Self { quiet, loud }
    }

    fn from_cover(theme: &Theme, cover: [u8; 3]) -> Option<(Color, Color)> {
        let palette = theme.extended_palette();

        Some((
            accent::tinted(palette.primary.weak.color, cover)?,
            accent::tinted(palette.primary.strong.color, cover)?,
        ))
    }

    fn at(self, tint: Tint, amplitude: f32, index: usize, bars: usize) -> Color {
        let t = match tint {
            Tint::Flat => 0.0,
            Tint::Amplitude | Tint::Artwork => amplitude.clamp(0.0, 1.0),
            Tint::Spectrum => {
                if bars <= 1 {
                    0.0
                } else {
                    index as f32 / (bars - 1) as f32
                }
            }
        };

        Color::from_rgb(
            self.quiet.r + (self.loud.r - self.quiet.r) * t,
            self.quiet.g + (self.loud.g - self.quiet.g) * t,
            self.quiet.b + (self.loud.b - self.quiet.b) * t,
        )
    }
}

#[cfg(test)]
fn bar_color(
    theme: &Theme,
    tint: Tint,
    cover: Option<[u8; 3]>,
    amplitude: f32,
    index: usize,
    bars: usize,
) -> Color {
    Ramp::new(theme, tint, cover).at(tint, amplitude, index, bars)
}

impl<Message> Widget<Message, Theme, Renderer> for Spectrum {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        _shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if let Event::Window(iced::window::Event::RedrawRequested(_)) = event {
            let state = tree.state.downcast_mut::<State>();

            if self.settings.peak_hold {
                let bars = bar_count(layout.bounds().width, self.settings.density.pitch());
                state.advance(&self.bins, bars);
            } else {
                state.forget();
            }
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;

        let bounds = layout.bounds();
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return;
        }

        let bars = bar_count(bounds.width, self.settings.density.pitch());
        let markers = tree.state.downcast_ref::<State>().markers();
        let ramp = Ramp::new(theme, self.settings.tint, self.cover);
        let text = theme.extended_palette().background.base.text;

        for index in 0..bars {
            let amplitude = amplitude(&self.bins, index, bars);
            let rect = bar_rect(bounds, index, bars, amplitude);
            let color = ramp.at(self.settings.tint, amplitude, index, bars);

            renderer.fill_quad(
                Quad {
                    bounds: rect,
                    border: Border {
                        radius: self.settings.caps.radius(rect.width, rect.height),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                Background::Color(color),
            );

            if let Some(peak) = markers.get(index)
                && let Some(marker) = peak_rect(bounds, rect, peak.level)
            {
                renderer.fill_quad(
                    Quad {
                        bounds: marker,
                        border: Border {
                            radius: self.settings.caps.radius(marker.width, marker.height),
                            ..Border::default()
                        },
                        ..Quad::default()
                    },
                    Background::Color(lift_toward(color, text, PEAK_LIFT)),
                );
            }
        }
    }
}

fn lift_toward(from: Color, toward: Color, t: f32) -> Color {
    Color {
        r: from.r + (toward.r - from.r) * t,
        g: from.g + (toward.g - from.g) * t,
        b: from.b + (toward.b - from.b) * t,
        a: 1.0,
    }
}

#[cfg(test)]
fn peak_color(theme: &Theme, bar: Color) -> Color {
    lift_toward(
        bar,
        theme.extended_palette().background.base.text,
        PEAK_LIFT,
    )
}

fn peak_rect(bounds: Rectangle, bar: Rectangle, peak: f32) -> Option<Rectangle> {
    let height = (peak.clamp(0.0, 1.0) * bounds.height).max(FLOOR);
    let y = bounds.y + bounds.height - height;

    (y + PEAK_THICKNESS + PEAK_GAP <= bar.y).then_some(Rectangle {
        x: bar.x,
        y,
        width: bar.width,
        height: PEAK_THICKNESS,
    })
}

impl<'a, Message: 'a> From<Spectrum> for Element<'a, Message, Theme, Renderer> {
    fn from(spectrum: Spectrum) -> Self {
        Self::new(spectrum)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::artwork::palette;
    use crate::layout::MIN_PANE;
    use crate::pane::settings::Density;

    fn bounds(width: f32, height: f32) -> Rectangle {
        Rectangle {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
    }

    const PITCH: f32 = 12.0;

    fn ramp() -> [f32; NUM_BINS] {
        let mut bins = [0.0; NUM_BINS];
        for (i, bin) in bins.iter_mut().enumerate() {
            *bin = i as f32 / (NUM_BINS - 1) as f32;
        }
        bins
    }

    #[test]
    fn a_wide_pane_stops_at_the_bins_the_analyzer_resolves() {
        assert_eq!(bar_count(4_000.0, PITCH), NUM_BINS);
    }

    fn counts(width: f32) -> Vec<usize> {
        Density::ALL
            .iter()
            .map(|density| bar_count(width, density.pitch()))
            .collect()
    }

    #[test]
    fn the_densities_stay_apart_across_the_widths_panes_actually_get() {
        for width in [200.0, 300.0, 400.0, 640.0] {
            let counts = counts(width);
            assert!(
                counts[0] > counts[1] && counts[1] > counts[2],
                "at {width}px the densities collapsed to {counts:?}"
            );
        }
    }

    #[test]
    fn a_coarser_setting_never_asks_for_more_bars() {
        for step in 0..400 {
            let (width, counts) = (step as f32 * 8.0, counts(step as f32 * 8.0));
            assert!(
                counts[0] >= counts[1] && counts[1] >= counts[2],
                "at {width}px a coarser setting drew more bars: {counts:?}"
            );
        }
    }

    #[test]
    fn only_the_finest_setting_saturates_on_a_wide_pane() {
        let counts = counts(1_280.0);
        assert_eq!(
            counts[0], NUM_BINS,
            "the finest setting should spend every bin the analyzer resolves"
        );
        assert!(
            counts[2] < NUM_BINS,
            "a coarse pane at 1280px still hit the ceiling, so the setting stops meaning anything"
        );
    }

    #[test]
    fn a_tiny_pane_falls_back_to_three_blocks() {
        for width in [MIN_PANE, 60.0, COMPACT_WIDTH - 1.0] {
            assert_eq!(
                bar_count(width, PITCH),
                COMPACT_BARS,
                "a {width}px pane should collapse rather than draw hairlines"
            );
        }
    }

    #[test]
    fn every_density_collapses_at_the_same_width() {
        for density in Density::ALL {
            assert_eq!(
                bar_count(COMPACT_WIDTH - 1.0, density.pitch()),
                COMPACT_BARS,
                "{density:?} did not collapse with the others"
            );
            assert!(
                bar_count(COMPACT_WIDTH, density.pitch()) >= MIN_BARS,
                "{density:?} was still collapsed past the cutoff"
            );
        }
    }

    #[test]
    fn the_three_blocks_split_the_spectrum_evenly() {
        let sizes: Vec<usize> = (0..COMPACT_BARS)
            .map(|index| {
                let (start, end) = group(index, COMPACT_BARS);
                end - start
            })
            .collect();

        let (small, large) = (
            sizes.iter().min().copied().expect("a block"),
            sizes.iter().max().copied().expect("a block"),
        );
        assert!(
            large - small <= 1,
            "the compact blocks split the bins {sizes:?} rather than into thirds"
        );
    }

    #[test]
    fn a_collapsed_pane_still_draws_bars_worth_looking_at() {
        let area = bounds(MIN_PANE, 40.0);
        let bars = bar_count(area.width, PITCH);

        for index in 0..bars {
            let rect = bar_rect(area, index, bars, 1.0);
            assert!(
                rect.width >= 4.0,
                "block {index} was only {}px wide in a collapsed pane",
                rect.width
            );
        }
    }

    #[test]
    fn nonsense_widths_still_give_a_count() {
        assert_eq!(bar_count(f32::NAN, PITCH), COMPACT_BARS);
        assert_eq!(bar_count(-100.0, PITCH), COMPACT_BARS);
        assert_eq!(bar_count(0.0, PITCH), COMPACT_BARS);
        assert_eq!(bar_count(f32::INFINITY, PITCH), COMPACT_BARS);
    }

    #[test]
    fn the_count_never_falls_outside_the_range() {
        for width in [0.0, MIN_PANE, 100.0, 333.0, 1_920.0] {
            let bars = bar_count(width, PITCH);
            assert!(
                (COMPACT_BARS..=NUM_BINS).contains(&bars),
                "width {width} gave {bars} bars"
            );
        }
    }

    #[test]
    fn a_wider_pane_never_shows_fewer_bars() {
        let mut last = 0;
        for step in 0..200 {
            let bars = bar_count(step as f32 * 10.0, PITCH);
            assert!(bars >= last, "bar count fell going wider at step {step}");
            last = bars;
        }
    }

    #[test]
    fn a_bar_is_always_at_least_a_pixel_wide() {
        for width in [MIN_PANE, 60.0, 120.0, 400.0, 1_600.0] {
            let bars = bar_count(width, PITCH);
            let rect = bar_rect(bounds(width, 80.0), 0, bars, 0.5);
            assert!(
                rect.width >= MIN_BAR,
                "width {width} gave a {}px bar",
                rect.width
            );
        }
    }

    /// `group` divides by the bar count, so a zero would panic rather than
    /// misdraw. No caller can produce one today; the floor is what keeps that
    /// from becoming a crash if one ever does.
    #[test]
    fn a_group_of_no_bars_does_not_divide_by_zero() {
        let (start, end) = group(0, 0);
        assert!(start < end && end <= NUM_BINS);
        assert_eq!(amplitude(&[0.5; NUM_BINS], 0, 0), 0.5);
    }

    /// Every group has to be a valid slice of the bins, at every count the
    /// widget can reach, or `amplitude` panics on a range it cannot index.
    #[test]
    fn every_group_is_a_slice_that_exists() {
        for bars in 1..=NUM_BINS {
            for index in 0..bars {
                let (start, end) = group(index, bars);
                assert!(
                    start < end && end <= NUM_BINS,
                    "{bars} bars gave bar {index} the range {start}..{end}"
                );
            }
        }
    }

    #[test]
    fn the_groups_tile_every_bin_exactly_once() {
        for bars in MIN_BARS..=NUM_BINS {
            let mut covered = 0;
            let mut expected_start = 0;
            for index in 0..bars {
                let (start, end) = group(index, bars);
                assert_eq!(start, expected_start, "{bars} bars left a gap or overlap");
                covered += end - start;
                expected_start = end;
            }
            assert_eq!(covered, NUM_BINS, "{bars} bars did not cover every bin");
        }
    }

    #[test]
    fn no_bar_carries_more_than_one_bin_over_another() {
        for bars in MIN_BARS..=NUM_BINS {
            let sizes: Vec<usize> = (0..bars)
                .map(|index| {
                    let (start, end) = group(index, bars);
                    end - start
                })
                .collect();
            let (small, large) = (
                sizes.iter().min().copied().expect("at least one bar"),
                sizes.iter().max().copied().expect("at least one bar"),
            );
            assert!(
                large - small <= 1,
                "{bars} bars ranged from {small} to {large} bins per bar"
            );
        }
    }

    #[test]
    fn folding_takes_the_peak_not_the_average() {
        let mut bins = [0.0; NUM_BINS];
        bins[0] = 1.0;

        assert_eq!(
            amplitude(&bins, 0, MIN_BARS),
            1.0,
            "a transient should keep its height when bins are folded together"
        );
    }

    #[test]
    fn a_full_width_bar_maps_to_exactly_one_bin() {
        let bins = ramp();
        for index in 0..NUM_BINS {
            assert_eq!(amplitude(&bins, index, NUM_BINS), bins[index]);
        }
    }

    #[test]
    fn the_shape_survives_being_folded_down() {
        let bins = ramp();
        for bars in MIN_BARS..=NUM_BINS {
            let mut last = -1.0;
            for index in 0..bars {
                let level = amplitude(&bins, index, bars);
                assert!(
                    level > last,
                    "{bars} bars lost the ramp's ordering at bar {index}"
                );
                last = level;
            }
        }
    }

    #[test]
    fn silence_still_draws_a_resting_line() {
        let rect = bar_rect(bounds(200.0, 100.0), 0, 16, 0.0);
        assert_eq!(rect.height, FLOOR);
        assert_eq!(rect.y, 100.0 - FLOOR, "the floor should sit on the bottom");
    }

    #[test]
    fn a_full_scale_bar_fills_the_pane_without_escaping_it() {
        let rect = bar_rect(bounds(200.0, 100.0), 0, 16, 1.0);
        assert_eq!(rect.height, 100.0);
        assert_eq!(rect.y, 0.0);
    }

    #[test]
    fn a_bar_never_grows_past_the_top() {
        let rect = bar_rect(bounds(200.0, 100.0), 0, 16, 4.0);
        assert!(rect.height <= 100.0, "an over-range bin escaped the bounds");
        assert!(rect.y >= 0.0);
    }

    #[test]
    fn a_pane_too_short_for_the_floor_still_stays_inside_it() {
        let rect = bar_rect(bounds(200.0, 1.0), 0, 16, 0.0);
        assert!(
            rect.height <= 1.0 && rect.y >= 0.0,
            "the resting line escaped a {}px pane",
            1.0
        );
    }

    #[test]
    fn the_bars_stay_inside_the_pane_across_the_row() {
        for width in [MIN_PANE, 150.0, 640.0] {
            let area = bounds(width, 90.0);
            let bars = bar_count(width, PITCH);
            for index in 0..bars {
                let rect = bar_rect(area, index, bars, 1.0);
                assert!(
                    rect.x >= area.x - f32::EPSILON,
                    "bar {index} started left of the pane at width {width}"
                );
                assert!(
                    rect.x + rect.width <= area.x + area.width + f32::EPSILON,
                    "bar {index} ran past the right edge at width {width}"
                );
            }
        }
    }

    #[test]
    fn the_bars_do_not_overlap_each_other() {
        let area = bounds(400.0, 90.0);
        let bars = bar_count(area.width, PITCH);
        for index in 1..bars {
            let previous = bar_rect(area, index - 1, bars, 1.0);
            let current = bar_rect(area, index, bars, 1.0);
            assert!(
                previous.x + previous.width <= current.x + f32::EPSILON,
                "bar {index} overlapped the one before it"
            );
        }
    }

    fn rgb(color: Color) -> (f32, f32, f32) {
        (color.r, color.g, color.b)
    }

    #[test]
    fn tinting_by_level_makes_amplitude_visible_in_the_color() {
        let theme = Theme::Dark;
        assert_ne!(
            rgb(bar_color(&theme, Tint::Amplitude, None, 0.0, 0, 8)),
            rgb(bar_color(&theme, Tint::Amplitude, None, 1.0, 0, 8)),
            "amplitude should be visible in the color, not only the height"
        );
    }

    #[test]
    fn the_color_comes_from_the_palette_in_both_themes() {
        for theme in [Theme::Light, Theme::Dark] {
            let palette = theme.extended_palette();
            assert_eq!(
                rgb(bar_color(&theme, Tint::Amplitude, None, 0.0, 0, 8)),
                rgb(palette.primary.weak.color),
                "a resting bar should be the theme's weak primary"
            );
        }
    }

    #[test]
    fn a_flat_tint_ignores_the_level() {
        let theme = Theme::Dark;
        assert_eq!(
            rgb(bar_color(&theme, Tint::Flat, None, 0.0, 0, 8)),
            rgb(bar_color(&theme, Tint::Flat, None, 1.0, 0, 8)),
        );
    }

    #[test]
    fn a_spectrum_tint_follows_position_rather_than_level() {
        let theme = Theme::Dark;
        assert_eq!(
            rgb(bar_color(&theme, Tint::Spectrum, None, 0.1, 3, 8)),
            rgb(bar_color(&theme, Tint::Spectrum, None, 0.9, 3, 8)),
            "level should not reach the color when tinting by frequency"
        );
        assert_ne!(
            rgb(bar_color(&theme, Tint::Spectrum, None, 0.5, 0, 8)),
            rgb(bar_color(&theme, Tint::Spectrum, None, 0.5, 7, 8)),
            "the two ends of the row should not be the same color"
        );
    }

    #[test]
    fn a_full_scale_bar_reaches_the_end_of_its_ramp() {
        for theme in [Theme::Light, Theme::Dark] {
            assert_eq!(
                rgb(bar_color(&theme, Tint::Amplitude, None, 1.0, 0, 8)),
                rgb(theme.extended_palette().primary.strong.color),
                "the loud end of the ramp should be reachable"
            );
        }
    }

    #[test]
    fn the_frequency_ramp_is_visible_in_every_theme_on_offer() {
        for theme in crate::config::ALL_THEMES {
            let (start, end) = (
                bar_color(theme, Tint::Spectrum, None, 0.5, 0, 16),
                bar_color(theme, Tint::Spectrum, None, 0.5, 15, 16),
            );
            let distance =
                (start.r - end.r).abs() + (start.g - end.g).abs() + (start.b - end.b).abs();

            assert!(
                distance > 0.25,
                "{theme} ramps between two colors that read as one"
            );
        }
    }

    #[test]
    fn a_single_bar_does_not_divide_by_zero() {
        let color = bar_color(&Theme::Dark, Tint::Spectrum, None, 0.5, 0, 1);
        assert!(color.r.is_finite() && color.g.is_finite() && color.b.is_finite());
    }

    const RED_SLEEVE: [u8; 3] = [150, 40, 40];
    const BLUE_SLEEVE: [u8; 3] = [40, 50, 160];

    #[test]
    fn an_artwork_tint_takes_the_hue_of_the_cover() {
        let red = bar_color(&Theme::Dark, Tint::Artwork, Some(RED_SLEEVE), 1.0, 0, 8);
        let blue = bar_color(&Theme::Dark, Tint::Artwork, Some(BLUE_SLEEVE), 1.0, 0, 8);

        assert!(red.r > red.b, "a red sleeve did not give red bars");
        assert!(blue.b > blue.r, "a blue sleeve did not give blue bars");
    }

    /// The cover color is fitted to sit behind text, so using it directly would
    /// draw bars barely distinguishable from a dark theme's own background.
    #[test]
    fn artwork_bars_stay_as_readable_as_the_theme_ramp() {
        for theme in crate::config::ALL_THEMES {
            for sleeve in [RED_SLEEVE, BLUE_SLEEVE] {
                let tinted = bar_color(theme, Tint::Artwork, Some(sleeve), 1.0, 0, 8);
                let default = bar_color(theme, Tint::Amplitude, None, 1.0, 0, 8);

                let lightness = |color: Color| {
                    let bytes = [color.r, color.g, color.b]
                        .map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8);
                    palette::to_hsl(bytes).2
                };
                assert!(
                    (lightness(tinted) - lightness(default)).abs() < 0.05,
                    "{theme} draws artwork bars at a different depth than its own ramp, \
                     so one of the two is wrong against the background"
                );
            }
        }
    }

    #[test]
    fn an_artwork_tint_still_ramps_with_the_level() {
        let quiet = bar_color(&Theme::Dark, Tint::Artwork, Some(RED_SLEEVE), 0.0, 0, 8);
        let loud = bar_color(&Theme::Dark, Tint::Artwork, Some(RED_SLEEVE), 1.0, 0, 8);

        assert_ne!(
            rgb(quiet),
            rgb(loud),
            "the level stopped being visible once the cover supplied the hue"
        );
    }

    #[test]
    fn a_cover_with_no_hue_falls_back_to_the_theme() {
        for gray in [[20u8, 20, 20], [128, 128, 128], [240, 240, 240]] {
            assert_eq!(
                rgb(bar_color(
                    &Theme::Dark,
                    Tint::Artwork,
                    Some(gray),
                    1.0,
                    0,
                    8
                )),
                rgb(bar_color(&Theme::Dark, Tint::Amplitude, None, 1.0, 0, 8)),
                "a gray sleeve tinted the bars to an arbitrary hue"
            );
        }
    }

    #[test]
    fn nothing_playing_falls_back_to_the_theme() {
        for theme in [Theme::Light, Theme::Dark] {
            assert_eq!(
                rgb(bar_color(&theme, Tint::Artwork, None, 1.0, 0, 8)),
                rgb(bar_color(&theme, Tint::Amplitude, None, 1.0, 0, 8)),
                "a pane with no cover to read drew something other than the theme ramp"
            );
        }
    }

    #[test]
    fn every_density_tiles_the_pane_at_every_size() {
        for density in Density::ALL {
            for width in [MIN_PANE, 80.0, 200.0, 640.0, 1_920.0] {
                let area = bounds(width, 90.0);
                let bars = bar_count(width, density.pitch());
                let last = bar_rect(area, bars - 1, bars, 1.0);

                assert!(
                    last.x + last.width <= area.x + area.width + f32::EPSILON,
                    "{density:?} at {width}px ran {} past the right edge",
                    last.x + last.width - area.width
                );
                assert!(
                    bar_rect(area, 0, bars, 1.0).width >= MIN_BAR,
                    "{density:?} at {width}px gave a sub-pixel bar"
                );
            }
        }
    }

    fn distance(a: Color, b: Color) -> f32 {
        (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs()
    }

    #[test]
    fn a_peak_marker_stands_out_from_the_bar_it_sits_above() {
        for theme in crate::config::ALL_THEMES {
            for tint in Tint::ALL {
                let bar = bar_color(theme, tint, None, 1.0, 0, 8);
                assert!(
                    distance(peak_color(theme, bar), bar) > 0.1,
                    "{tint:?} draws a marker too close to the bar's own color to see"
                );
            }
        }
    }

    #[test]
    fn a_peak_marker_stays_inside_the_spectrum_palette() {
        for theme in crate::config::ALL_THEMES {
            for tint in Tint::ALL {
                let bar = bar_color(theme, tint, None, 1.0, 0, 8);
                let text = theme.extended_palette().background.base.text;
                assert!(
                    distance(peak_color(theme, bar), bar) < distance(text, bar),
                    "{tint:?} draws its marker as far off as bare chrome would"
                );
            }
        }
    }

    #[test]
    fn a_peak_marker_sits_above_the_bar_it_follows() {
        let area = bounds(200.0, 100.0);
        let bar = bar_rect(area, 0, 8, 0.2);
        let marker = peak_rect(area, bar, 0.8).expect("a peak well above the bar draws");

        assert!(
            marker.y + marker.height <= bar.y,
            "the marker overlapped the bar"
        );
        assert!(marker.y >= area.y, "the marker escaped the top of the pane");
    }

    #[test]
    fn a_peak_inside_its_bar_draws_nothing() {
        let area = bounds(200.0, 100.0);
        let bar = bar_rect(area, 0, 8, 0.8);
        assert!(peak_rect(area, bar, 0.8).is_none());
        assert!(peak_rect(area, bar, 0.5).is_none());
    }

    #[test]
    fn a_peak_marker_is_as_wide_as_its_bar() {
        let area = bounds(200.0, 100.0);
        let bar = bar_rect(area, 2, 8, 0.1);
        let marker = peak_rect(area, bar, 0.9).expect("a peak above the bar draws");

        assert!((marker.width - bar.width).abs() < f32::EPSILON);
        assert!((marker.x - bar.x).abs() < f32::EPSILON);
    }

    /// The marker state is fixed-size on purpose; a regression to a heap
    /// collection would not fail any behavioral test above, so its footprint is
    /// pinned here instead.
    #[test]
    fn the_marker_state_holds_no_heap_allocation() {
        assert_eq!(
            std::mem::size_of::<State>(),
            std::mem::size_of::<[Peak; NUM_BINS]>() + std::mem::size_of::<usize>(),
            "State grew a pointer, so the markers are back on the heap"
        );
    }

    fn level(state: &State, index: usize) -> f32 {
        state.markers()[index].level
    }

    fn settle(state: &mut State, bins: &[f32; NUM_BINS], bars: usize, frames: usize) {
        for _ in 0..frames {
            state.advance(bins, bars);
        }
    }

    #[test]
    fn peaks_fall_toward_the_level_without_passing_it() {
        let mut state = State::default();
        let quiet = [0.0; NUM_BINS];

        state.advance(&[1.0; NUM_BINS], 8);
        assert!((level(&state, 0) - 1.0).abs() < f32::EPSILON);

        settle(&mut state, &quiet, 8, PEAK_HANG as usize + 60);
        let after = level(&state, 0);
        assert!(after < 1.0, "the peak did not fall");
        assert!(
            after >= 0.0,
            "the peak fell past the level it was following"
        );
    }

    #[test]
    fn a_peak_holds_still_before_it_starts_to_fall() {
        let mut state = State::default();
        state.advance(&[1.0; NUM_BINS], 8);

        settle(&mut state, &[0.0; NUM_BINS], 8, PEAK_HANG as usize);

        assert!(
            (level(&state, 0) - 1.0).abs() < f32::EPSILON,
            "the marker started falling before it had been still long enough to read"
        );
    }

    #[test]
    fn a_peak_is_still_readable_a_second_after_the_note() {
        let mut state = State::default();
        state.advance(&[1.0; NUM_BINS], 8);

        settle(&mut state, &[0.0; NUM_BINS], 8, 60);

        assert!(
            level(&state, 0) > 0.5,
            "a second later the marker had fallen to {} â€” the hold does not read",
            level(&state, 0)
        );
    }

    #[test]
    fn a_peak_the_music_left_behind_does_catch_up_eventually() {
        let mut state = State::default();
        state.advance(&[1.0; NUM_BINS], 8);

        settle(&mut state, &[0.0; NUM_BINS], 8, 600);

        assert!(
            level(&state, 0) < 0.05,
            "a marker was still hanging at {} ten seconds into silence",
            level(&state, 0)
        );
    }

    #[test]
    fn a_peak_rises_to_a_new_level_at_once() {
        let mut state = State::default();
        state.advance(&[0.0; NUM_BINS], 8);
        state.advance(&[1.0; NUM_BINS], 8);

        assert!(
            (level(&state, 0) - 1.0).abs() < f32::EPSILON,
            "a peak should follow a transient up without lag"
        );
    }

    #[test]
    fn switching_peak_hold_off_forgets_the_markers_it_was_holding() {
        let mut state = State::default();
        state.advance(&[1.0; NUM_BINS], 8);
        state.forget();

        assert!(
            state.markers().is_empty(),
            "markers from before the setting was switched off survived it"
        );
    }

    #[test]
    fn coming_back_to_peak_hold_starts_from_the_audio_not_the_past() {
        let mut state = State::default();
        state.advance(&[1.0; NUM_BINS], 8);
        state.forget();
        state.advance(&[0.1; NUM_BINS], 8);

        assert!(
            state.markers().iter().all(|peak| peak.level < 0.2),
            "a stale marker dropped in from before the setting was switched off"
        );
    }

    #[test]
    fn resizing_seeds_the_new_bars_from_the_audio() {
        let mut state = State::default();
        let bins = [0.7; NUM_BINS];

        state.advance(&bins, 4);
        state.advance(&bins, 16);

        assert_eq!(state.markers().len(), 16);
        assert!(
            state.markers().iter().all(|peak| peak.level > 0.0),
            "a new bar started from silence"
        );
    }
}
