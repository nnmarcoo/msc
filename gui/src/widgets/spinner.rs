//! An indeterminate progress ring.
//!
//! Ported from bloom's `loading_spinner`, reduced to the circular form and
//! restyled the way the rest of verse's widgets are: a closure against the
//! theme rather than a `StyleSheet` trait, matching [`super::marquee`] and
//! [`crate::styles`].
//!
//! It answers "still working" where a fixed label cannot. "Loading…" is the same
//! pixels whether a request is a hundred milliseconds from returning or has
//! silently died, so a stalled pane and a busy one look identical; a ring that
//! is still turning is evidence the frame loop is alive and the work is
//! outstanding.
//!
//! The arc both rotates and changes length, which is what separates it from a
//! spinning dash. A constant-length arc at constant speed has no feature the eye
//! can hold onto, so it reads as a texture rather than motion; growing from
//! [`MIN_ANGLE`] to nearly a full turn and shrinking back gives it a head and a
//! tail that visibly travel.
//!
//! Rotation is counted in `u32` steps that wrap rather than radians that grow.
//! An `f32` angle accumulating every frame loses precision as it climbs — after
//! an hour of a visible spinner the increments no longer land on distinct
//! values and the ring stutters — where a wrapping integer is exact forever and
//! costs one modulo when it is read.
//!
//! Drawing is a canvas rather than [`iced::advanced::renderer::Quad`]s, which
//! every other widget here uses. An arc is the one shape quads cannot express:
//! approximating it means a fan of tiny rotated rectangles, and rotation is
//! exactly what a quad has no way to state. This is why the crate carries the
//! `canvas` feature at all.
//!
//! The geometry is cached and invalidated once per animated frame. The cache
//! earns its place on resize and on repaints the animation did not cause, where
//! the ring is redrawn from the same tessellation instead of re-walking the
//! path.

use std::f32::consts::PI;
use std::time::Duration;

use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer};
use iced::time::Instant;
use iced::widget::canvas;
use iced::{
    Color, Element, Event, Length, Radians, Rectangle, Renderer, Size, Theme, Vector, mouse, window,
};

/// How much of the ring is drawn at its shortest.
///
/// Zero would make the arc vanish at each turn of the cycle and read as a
/// flicker rather than a rotation.
const MIN_ANGLE: Radians = Radians(PI / 8.0);

/// How much the arc grows across one cycle.
///
/// Deliberately short of a full turn: an arc that closes into a complete circle
/// has no ends, so the rotation becomes invisible for the moment it is closed.
const WRAP_ANGLE: Radians = Radians(2.0 * PI - PI / 4.0);

const BASE_ROTATION_SPEED: u32 = u32::MAX / 80;

const DEFAULT_SIZE: f32 = 24.0;
const DEFAULT_BAR: f32 = 2.5;
const DEFAULT_CYCLE: Duration = Duration::from_millis(600);
const DEFAULT_ROTATION: Duration = Duration::from_secs(2);

fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - ((-2.0 * t + 2.0).powi(3)) / 2.0
    }
}

pub fn spinner<'a>() -> Spinner<'a> {
    Spinner::new()
}

type Styler<'a> = Box<dyn Fn(&Theme) -> Color + 'a>;

pub struct Spinner<'a> {
    size: f32,
    bar_height: f32,
    style: Option<Styler<'a>>,
    cycle_duration: Duration,
    rotation_duration: Duration,
}

impl<'a> Spinner<'a> {
    pub fn new() -> Self {
        Self {
            size: DEFAULT_SIZE,
            bar_height: DEFAULT_BAR,
            style: None,
            cycle_duration: DEFAULT_CYCLE / 2,
            rotation_duration: DEFAULT_ROTATION,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn bar_height(mut self, height: f32) -> Self {
        self.bar_height = height;
        self
    }

    pub fn style(mut self, style: impl Fn(&Theme) -> Color + 'a) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub fn cycle_duration(mut self, duration: Duration) -> Self {
        self.cycle_duration = duration / 2;
        self
    }

    pub fn rotation_duration(mut self, duration: Duration) -> Self {
        self.rotation_duration = duration;
        self
    }

    fn color(&self, theme: &Theme) -> Color {
        match &self.style {
            Some(style) => style(theme),
            None => theme.extended_palette().primary.base.color,
        }
    }
}

impl Default for Spinner<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
enum Animation {
    Expanding {
        start: Instant,
        progress: f32,
        rotation: u32,
        last: Instant,
    },
    Contracting {
        start: Instant,
        progress: f32,
        rotation: u32,
        last: Instant,
    },
}

impl Default for Animation {
    fn default() -> Self {
        Self::Expanding {
            start: Instant::now(),
            progress: 0.0,
            rotation: 0,
            last: Instant::now(),
        }
    }
}

impl Animation {
    fn next(self, additional_rotation: u32, now: Instant) -> Self {
        match self {
            Self::Expanding { rotation, .. } => Self::Contracting {
                start: now,
                progress: 0.0,
                rotation: rotation.wrapping_add(additional_rotation),
                last: now,
            },
            Self::Contracting { rotation, .. } => Self::Expanding {
                start: now,
                progress: 0.0,
                rotation: rotation.wrapping_add(BASE_ROTATION_SPEED.wrapping_add(
                    (f64::from(WRAP_ANGLE.0 / (2.0 * PI)) * f64::from(u32::MAX)) as u32,
                )),
                last: now,
            },
        }
    }

    fn start(self) -> Instant {
        match self {
            Self::Expanding { start, .. } | Self::Contracting { start, .. } => start,
        }
    }

    fn last(self) -> Instant {
        match self {
            Self::Expanding { last, .. } | Self::Contracting { last, .. } => last,
        }
    }

    fn timed_transition(
        self,
        cycle_duration: Duration,
        rotation_duration: Duration,
        now: Instant,
    ) -> Self {
        let elapsed = now.duration_since(self.start());
        let additional_rotation = ((now - self.last()).as_secs_f32()
            / rotation_duration.as_secs_f32()
            * u32::MAX as f32) as u32;

        if elapsed > cycle_duration {
            self.next(additional_rotation, now)
        } else {
            self.with_elapsed(cycle_duration, additional_rotation, elapsed, now)
        }
    }

    fn with_elapsed(
        self,
        cycle_duration: Duration,
        additional_rotation: u32,
        elapsed: Duration,
        now: Instant,
    ) -> Self {
        let progress = elapsed.as_secs_f32() / cycle_duration.as_secs_f32();
        let eased = ease_in_out_cubic(progress);

        match self {
            Self::Expanding {
                start, rotation, ..
            } => Self::Expanding {
                start,
                progress: eased,
                rotation: rotation.wrapping_add(additional_rotation),
                last: now,
            },
            Self::Contracting {
                start, rotation, ..
            } => Self::Contracting {
                start,
                progress: eased,
                rotation: rotation.wrapping_add(additional_rotation),
                last: now,
            },
        }
    }

    fn rotation(self) -> f32 {
        match self {
            Self::Expanding { rotation, .. } | Self::Contracting { rotation, .. } => {
                rotation as f32 / u32::MAX as f32
            }
        }
    }
}

#[derive(Default)]
struct State {
    animation: Animation,
    cache: canvas::Cache,
}

impl<Message> Widget<Message, Theme, Renderer> for Spinner<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(self.size),
            height: Length::Fixed(self.size),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.size, self.size)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();

        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            state.animation =
                state
                    .animation
                    .timed_transition(self.cycle_duration, self.rotation_duration, *now);

            state.cache.clear();
            shell.request_redraw();
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

        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let color = self.color(theme);

        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            let radius = frame.width() / 2.0 - self.bar_height;
            let start = Radians(state.animation.rotation() * 2.0 * PI);

            let mut builder = canvas::path::Builder::new();

            match state.animation {
                Animation::Expanding { progress, .. } => builder.arc(canvas::path::Arc {
                    center: frame.center(),
                    radius,
                    start_angle: start,
                    end_angle: start + MIN_ANGLE + WRAP_ANGLE * progress,
                }),
                Animation::Contracting { progress, .. } => builder.arc(canvas::path::Arc {
                    center: frame.center(),
                    radius,
                    start_angle: start + WRAP_ANGLE * progress,
                    end_angle: start + MIN_ANGLE + WRAP_ANGLE,
                }),
            }

            frame.stroke(
                &builder.build(),
                canvas::Stroke::default()
                    .with_color(color)
                    .with_width(self.bar_height),
            );
        });

        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            use iced::advanced::graphics::geometry::Renderer as _;
            renderer.draw_geometry(geometry);
        });
    }
}

impl<'a, Message: 'a> From<Spinner<'a>> for Element<'a, Message, Theme, Renderer> {
    fn from(spinner: Spinner<'a>) -> Self {
        Self::new(spinner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_spans_the_whole_range() {
        assert!((ease_in_out_cubic(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((ease_in_out_cubic(1.0) - 1.0).abs() < f32::EPSILON);
        assert!((ease_in_out_cubic(0.5) - 0.5).abs() < 0.001);
    }

    #[test]
    fn easing_never_moves_backwards() {
        let mut previous = 0.0;

        for step in 0..=100 {
            let eased = ease_in_out_cubic(step as f32 / 100.0);
            assert!(eased >= previous, "eased dipped at {step}");
            previous = eased;
        }
    }

    /// The arc must keep two visible ends, or the rotation stops reading as
    /// motion at the moment it closes.
    const _: () = assert!(MIN_ANGLE.0 > 0.0 && WRAP_ANGLE.0 < 2.0 * PI);

    #[test]
    fn a_cycle_alternates_between_growing_and_shrinking() {
        let now = Instant::now();
        let expanding = Animation::default();

        assert!(matches!(expanding, Animation::Expanding { .. }));
        assert!(matches!(
            expanding.next(0, now),
            Animation::Contracting { .. }
        ));
        assert!(matches!(
            expanding.next(0, now).next(0, now),
            Animation::Expanding { .. }
        ));
    }

    /// Rotation is a wrapping counter precisely so a long-lived spinner cannot
    /// drift or stutter the way an accumulating float would.
    #[test]
    fn rotation_wraps_rather_than_growing_without_bound() {
        let now = Instant::now();
        let nearly_round = Animation::Expanding {
            start: now,
            progress: 0.0,
            rotation: u32::MAX - 10,
            last: now,
        };

        let rotation = nearly_round.next(100, now).rotation();

        assert!(
            (0.0..=1.0).contains(&rotation),
            "rotation left its range: {rotation}"
        );
    }

    #[test]
    fn a_rotation_reads_as_a_fraction_of_one_turn() {
        let now = Instant::now();

        let none = Animation::Expanding {
            start: now,
            progress: 0.0,
            rotation: 0,
            last: now,
        };
        let half = Animation::Expanding {
            start: now,
            progress: 0.0,
            rotation: u32::MAX / 2,
            last: now,
        };

        assert!((none.rotation() - 0.0).abs() < f32::EPSILON);
        assert!((half.rotation() - 0.5).abs() < 0.001);
    }
}
