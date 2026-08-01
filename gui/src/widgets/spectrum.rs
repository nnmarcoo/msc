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
//! Folding takes the maximum of each group, not the mean. Averaging a peak
//! against its quiet neighbours pulls transients down, and the visible result is
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
//! Every bar keeps [`FLOOR`] of height at silence. A spectrum that renders
//! nothing at all is indistinguishable from a pane that has failed to draw, and
//! the resting line also gives the rounded caps something to sit on.
//!
//! The widget paints no background. The pane chrome owns that, and the previous
//! version filling its own bounds is what made it fight the rounded corners.
//! Colour comes from the palette for the same reason: a hardcoded ramp toward
//! grey inverted its meaning under a light theme. Every tint interpolates
//! between two palette colours, so all three stay part of the theme; what
//! differs is only what drives the interpolation — nothing, the bar's level, or
//! its position across the row. `Flat` sits at the strong tone rather than the
//! weak one, since with no ramp to climb the weak end leaves every bar washed
//! out, and `Spectrum` divides by one less than the count, so a lone bar takes
//! the start of the ramp rather than dividing by zero.
//!
//! Peak markers are drawn in the palette's text colour rather than the bar's
//! own. A marker in the bar's colour is invisible under `Flat`, where every bar
//! is already exactly that colour, and only accidentally visible under the other
//! two. They are suppressed entirely while the peak is still inside its bar,
//! where the marker would read as a brighter stripe across the top rather than
//! as a marker.
//!
//! The markers live in the widget's own [`State`] because a marker is defined by
//! what the bars did *before* this frame, and they advance in `update` rather
//! than `draw`, which is handed that state immutably. The app already redraws
//! this pane on its animation tick whenever a track is loaded, so the markers
//! are paced by the same clock as the bars and nothing extra has to ask for a
//! frame; [`PEAK_FALL`] is therefore per frame rather than per second. A resize
//! seeds the new bars from what the audio is doing now, so widening a pane does
//! not drop a row of markers in from the floor.

use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout};
use iced::{
    Background, Border, Color, Element, Event, Length, Rectangle, Renderer, Size, Theme, mouse,
};

use verse_core::NUM_BINS;

use crate::pane::settings::{Tint, Visualizer};

const GAP_RATIO: f32 = 0.25;

const MIN_BARS: usize = 4;
const MIN_BAR: f32 = 1.0;

const FLOOR: f32 = 2.0;

const MIN_HEIGHT: f32 = 24.0;

const TINT: f32 = 0.55;

const PEAK_THICKNESS: f32 = 2.0;

const PEAK_FALL: f32 = 0.03;

pub struct Spectrum {
    bins: [f32; NUM_BINS],
    settings: Visualizer,
}

impl Spectrum {
    pub fn new(bins: [f32; NUM_BINS], settings: Visualizer) -> Self {
        Self { bins, settings }
    }

    pub const fn min_height() -> f32 {
        MIN_HEIGHT
    }
}

#[derive(Default)]
struct State {
    peaks: Vec<f32>,
}

impl State {
    fn advance(&mut self, bins: &[f32; NUM_BINS], bars: usize) {
        if self.peaks.len() != bars {
            self.peaks = (0..bars).map(|i| amplitude(bins, i, bars)).collect();
            return;
        }

        for (index, peak) in self.peaks.iter_mut().enumerate() {
            let level = amplitude(bins, index, bars);
            *peak = if level >= *peak {
                level
            } else {
                (*peak - PEAK_FALL).max(level)
            };
        }
    }
}

fn bar_count(width: f32, pitch: f32) -> usize {
    if !width.is_finite() || width <= 0.0 || pitch <= 0.0 {
        return MIN_BARS;
    }
    ((width / pitch) as usize).clamp(MIN_BARS, NUM_BINS)
}

fn group(index: usize, bars: usize) -> (usize, usize) {
    let start = index * NUM_BINS / bars;
    let end = (index + 1) * NUM_BINS / bars;
    (start, end.max(start + 1))
}

fn amplitude(bins: &[f32; NUM_BINS], index: usize, bars: usize) -> f32 {
    let (start, end) = group(index, bars);
    bins[start..end.min(NUM_BINS)]
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

fn bar_color(theme: &Theme, tint: Tint, amplitude: f32, index: usize, bars: usize) -> Color {
    let palette = theme.extended_palette();

    let t = match tint {
        Tint::Flat => 0.0,
        Tint::Amplitude => amplitude.clamp(0.0, 1.0) * TINT,
        Tint::Spectrum => {
            if bars <= 1 {
                0.0
            } else {
                index as f32 / (bars - 1) as f32
            }
        }
    };

    let (quiet, loud) = match tint {
        Tint::Flat => (palette.primary.strong.color, palette.primary.strong.color),
        Tint::Amplitude => (palette.primary.weak.color, palette.primary.strong.color),
        Tint::Spectrum => (palette.primary.strong.color, palette.success.strong.color),
    };

    Color::from_rgb(
        quiet.r + (loud.r - quiet.r) * t,
        quiet.g + (loud.g - quiet.g) * t,
        quiet.b + (loud.b - quiet.b) * t,
    )
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
        if !self.settings.peak_hold {
            return;
        }

        if let Event::Window(iced::window::Event::RedrawRequested(_)) = event {
            let bars = bar_count(layout.bounds().width, self.settings.density.pitch());
            tree.state.downcast_mut::<State>().advance(&self.bins, bars);
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
        let peaks = &tree.state.downcast_ref::<State>().peaks;

        for index in 0..bars {
            let amplitude = amplitude(&self.bins, index, bars);
            let rect = bar_rect(bounds, index, bars, amplitude);
            let color = bar_color(theme, self.settings.tint, amplitude, index, bars);

            renderer.fill_quad(
                Quad {
                    bounds: rect,
                    border: Border {
                        radius: self
                            .settings
                            .caps
                            .radius(rect.width)
                            .min(rect.height / 2.0)
                            .into(),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                Background::Color(color),
            );

            if self.settings.peak_hold
                && let Some(peak) = peaks.get(index)
                && let Some(marker) = peak_rect(bounds, rect, *peak)
            {
                renderer.fill_quad(
                    Quad {
                        bounds: marker,
                        border: Border {
                            radius: self
                                .settings
                                .caps
                                .radius(marker.width)
                                .min(marker.height / 2.0)
                                .into(),
                            ..Border::default()
                        },
                        ..Quad::default()
                    },
                    Background::Color(peak_color(theme)),
                );
            }
        }
    }
}

fn peak_color(theme: &Theme) -> Color {
    theme.extended_palette().background.base.text
}

fn peak_rect(bounds: Rectangle, bar: Rectangle, peak: f32) -> Option<Rectangle> {
    let height = (peak.clamp(0.0, 1.0) * bounds.height).max(FLOOR);
    let y = bounds.y + bounds.height - height;

    (y + PEAK_THICKNESS <= bar.y).then_some(Rectangle {
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
        assert_eq!(bar_count(2_000.0, PITCH), NUM_BINS);
        assert_eq!(bar_count(400.0, PITCH), NUM_BINS);
    }

    #[test]
    fn a_narrow_pane_keeps_a_readable_handful() {
        assert_eq!(bar_count(0.0, PITCH), MIN_BARS);
        assert_eq!(bar_count(1.0, PITCH), MIN_BARS);
    }

    #[test]
    fn nonsense_widths_still_give_a_count() {
        assert_eq!(bar_count(f32::NAN, PITCH), MIN_BARS);
        assert_eq!(bar_count(-100.0, PITCH), MIN_BARS);
        assert_eq!(bar_count(f32::INFINITY, PITCH), MIN_BARS);
    }

    #[test]
    fn the_count_never_falls_outside_the_range() {
        for width in [0.0, MIN_PANE, 100.0, 333.0, 1_920.0] {
            let bars = bar_count(width, PITCH);
            assert!(
                (MIN_BARS..=NUM_BINS).contains(&bars),
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
    fn tinting_by_level_makes_amplitude_visible_in_the_colour() {
        let theme = Theme::Dark;
        assert_ne!(
            rgb(bar_color(&theme, Tint::Amplitude, 0.0, 0, 8)),
            rgb(bar_color(&theme, Tint::Amplitude, 1.0, 0, 8)),
            "amplitude should be visible in the colour, not only the height"
        );
    }

    #[test]
    fn the_colour_comes_from_the_palette_in_both_themes() {
        for theme in [Theme::Light, Theme::Dark] {
            let palette = theme.extended_palette();
            assert_eq!(
                rgb(bar_color(&theme, Tint::Amplitude, 0.0, 0, 8)),
                rgb(palette.primary.weak.color),
                "a resting bar should be the theme's weak primary"
            );
        }
    }

    #[test]
    fn a_flat_tint_ignores_the_level() {
        let theme = Theme::Dark;
        assert_eq!(
            rgb(bar_color(&theme, Tint::Flat, 0.0, 0, 8)),
            rgb(bar_color(&theme, Tint::Flat, 1.0, 0, 8)),
        );
    }

    #[test]
    fn a_spectrum_tint_follows_position_rather_than_level() {
        let theme = Theme::Dark;
        assert_eq!(
            rgb(bar_color(&theme, Tint::Spectrum, 0.1, 3, 8)),
            rgb(bar_color(&theme, Tint::Spectrum, 0.9, 3, 8)),
            "level should not reach the colour when tinting by frequency"
        );
        assert_ne!(
            rgb(bar_color(&theme, Tint::Spectrum, 0.5, 0, 8)),
            rgb(bar_color(&theme, Tint::Spectrum, 0.5, 7, 8)),
            "the two ends of the row should not be the same colour"
        );
    }

    #[test]
    fn a_single_bar_does_not_divide_by_zero() {
        let color = bar_color(&Theme::Dark, Tint::Spectrum, 0.5, 0, 1);
        assert!(color.r.is_finite() && color.g.is_finite() && color.b.is_finite());
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

    #[test]
    fn a_peak_marker_does_not_match_the_bars_it_sits_above() {
        for theme in [Theme::Light, Theme::Dark] {
            for tint in Tint::ALL {
                assert_ne!(
                    rgb(peak_color(&theme)),
                    rgb(bar_color(&theme, tint, 1.0, 0, 8)),
                    "{tint:?} draws its marker in the bar's own colour"
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

    #[test]
    fn peaks_fall_toward_the_level_without_passing_it() {
        let mut state = State::default();
        let loud = [1.0; NUM_BINS];
        let quiet = [0.0; NUM_BINS];

        state.advance(&loud, 8);
        assert!((state.peaks[0] - 1.0).abs() < f32::EPSILON);

        state.advance(&quiet, 8);
        let after = state.peaks[0];
        assert!(after < 1.0, "the peak did not fall");
        assert!(after > 0.0, "the peak fell straight to the floor");
    }

    #[test]
    fn a_peak_rises_to_a_new_level_at_once() {
        let mut state = State::default();
        state.advance(&[0.0; NUM_BINS], 8);
        state.advance(&[1.0; NUM_BINS], 8);

        assert!(
            (state.peaks[0] - 1.0).abs() < f32::EPSILON,
            "a peak should follow a transient up without lag"
        );
    }

    #[test]
    fn resizing_seeds_the_new_bars_from_the_audio() {
        let mut state = State::default();
        let bins = [0.7; NUM_BINS];

        state.advance(&bins, 4);
        state.advance(&bins, 16);

        assert_eq!(state.peaks.len(), 16);
        assert!(
            state.peaks.iter().all(|peak| *peak > 0.0),
            "a new bar started from silence"
        );
    }
}
