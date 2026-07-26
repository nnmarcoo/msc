//! The seek bar: a scrubbable rail, and nothing else.
//!
//! A custom widget rather than a styled `slider` because a timeline answers two
//! questions at once, where playback *is* and where a click would take it, and
//! and a slider only draws the first. Hovering shows a ghost head at the cursor
//! so a seek can be aimed before it is committed. A slider would also tie the
//! hit target to the rail's thickness, and a 4px bar is not something anyone
//! can hit.
//!
//! So the rail takes a band [`SEEK_REACH`] taller than itself on each side, and
//! the widget is exactly that band, with no reserved room for labels. Everything
//! around it, including both clocks, belongs to [`crate::pane::timeline`],
//! which lays this out between its own rows. An earlier version drew the clocks
//! here on lines of their own while the pane stacked labels outside them, and
//! the doubled-up spacing left the title floating clear of the bar it named.
//! The widget claiming only the pixels it can be clicked on is what keeps the
//! rows tight against it.
//!
//! That makes the reach do double duty: it is both the margin a click may miss
//! the rail by and the entire gap between the rail and the pane's rows, since
//! the rows add no spacing of their own. Shrinking it therefore pulls the
//! labels closer, and its floor is [`HEAD_RADIUS`]: below that the widget is
//! shorter than the playhead and clips it.
//!
//! `Op::Hovered` reports the position under the pointer so the pane can show
//! the time a seek would land on. The widget keeps its own copy of that hover
//! for the ghost head, since `update` can request a redraw without re-running
//! the app's `view`, and a head that waited on a round trip through the app
//! would lag the cursor.
//!
//! The widget does not drive its own animation, and cannot. Requesting a redraw
//! from `update` repaints the tree that already exists rather than re-running
//! the app's `view`, so the rail redrew at full rate against whatever `position`
//! it had last been handed, giving smooth frames and a stale playhead. Motion needs a
//! new `position`, which only the app can supply, so the app's tick drives it.
//! See [`crate::app`].
//!
//! Dragging is tracked here but resolved by the app. `Op::Seek` fires
//! continuously while the pointer moves and `Op::Committed` once on release.
//! Only the second moves the audio: the app draws the rail from the stream of
//! `Seek`s, so the head stays under the pointer, and jumps playback once at the
//! end. Seeking on every move instead restarts the stream continuously and the
//! drag becomes an audible stutter.
//!
//! A press captures the pointer, so a drag that wanders outside the widget
//! keeps seeking, and releasing anywhere commits. That is why `dragging` lives in
//! widget state rather than being inferred from the cursor being over the rail,
//! and why the seek reach gates the press but never the drag.
//!
//! With no track loaded the rail still draws, empty and inert: a timeline that
//! vanishes between tracks makes the layout jump.

use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout};
use iced::{
    Background, Border, Color, Element, Event, Length, Rectangle, Renderer, Size, Theme, mouse,
};

const RAIL_HEIGHT: f32 = 4.0;
const HEAD_RADIUS: f32 = 5.0;
const SEEK_REACH: f32 = 5.0;
const MIN_HEIGHT: f32 = RAIL_HEIGHT + SEEK_REACH * 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Seek(f32),
    Committed,
    Hovered(Option<f32>),
}

#[derive(Default)]
struct State {
    dragging: bool,
    hovered: Option<f32>,
}

pub struct Timeline<'a, Message> {
    position: f32,
    duration: f32,
    on_op: Box<dyn Fn(Op) -> Message + 'a>,
}

impl<'a, Message> Timeline<'a, Message> {
    pub fn new(position: f32, duration: f32, on_op: impl Fn(Op) -> Message + 'a) -> Self {
        Self {
            position,
            duration,
            on_op: Box::new(on_op),
        }
    }

    fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            return 0.0;
        }
        (self.position / self.duration).clamp(0.0, 1.0)
    }

    fn is_live(&self) -> bool {
        self.duration > 0.0
    }

    fn rail(bounds: Rectangle) -> Rectangle {
        Rectangle {
            x: bounds.x,
            y: bounds.center_y() - RAIL_HEIGHT / 2.0,
            width: bounds.width.max(0.0),
            height: RAIL_HEIGHT,
        }
    }

    fn fraction_at(bounds: Rectangle, cursor: mouse::Cursor) -> Option<f32> {
        let position = cursor.position()?;
        let rail = Self::rail(bounds);
        if rail.width <= 0.0 {
            return None;
        }
        Some(((position.x - rail.x) / rail.width).clamp(0.0, 1.0))
    }

    fn seek_band(bounds: Rectangle) -> Rectangle {
        let rail = Self::rail(bounds);
        let top = (rail.y - SEEK_REACH).max(bounds.y);
        let bottom = (rail.y + rail.height + SEEK_REACH).min(bounds.y + bounds.height);
        Rectangle {
            x: bounds.x,
            y: top,
            width: bounds.width,
            height: (bottom - top).max(0.0),
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Timeline<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fixed(MIN_HEIGHT),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(Size::new(limits.max().width, MIN_HEIGHT))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let state = tree.state.downcast_mut::<State>();

        if !self.is_live() {
            state.dragging = false;
            state.hovered = None;
            return;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(Self::seek_band(bounds)) =>
            {
                if let Some(fraction) = Self::fraction_at(bounds, cursor) {
                    state.dragging = true;
                    state.hovered = Some(fraction);
                    shell.publish((self.on_op)(Op::Seek(fraction * self.duration)));
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let fraction = Self::fraction_at(bounds, cursor);

                if state.dragging {
                    if let Some(fraction) = fraction {
                        state.hovered = Some(fraction);
                        shell.publish((self.on_op)(Op::Seek(fraction * self.duration)));
                        shell.request_redraw();
                    }
                    return;
                }

                let over = cursor
                    .is_over(Self::seek_band(bounds))
                    .then_some(fraction)
                    .flatten();
                if over != state.hovered {
                    state.hovered = over;
                    shell.publish((self.on_op)(Op::Hovered(
                        over.map(|fraction| fraction * self.duration),
                    )));
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.dragging => {
                state.dragging = false;
                shell.publish((self.on_op)(Op::Committed));
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::CursorLeft)
                if !state.dragging && state.hovered.take().is_some() =>
            {
                shell.publish((self.on_op)(Op::Hovered(None)));
                shell.request_redraw();
            }
            _ => {}
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
        let bounds = layout.bounds();
        let palette = theme.extended_palette();
        let state = tree.state.downcast_ref::<State>();
        let rail = Self::rail(bounds);

        let progress = self.progress();
        let elapsed_width = rail.width * progress;

        fill(
            renderer,
            rail,
            palette.background.strong.color,
            RAIL_HEIGHT / 2.0,
        );

        if elapsed_width > 0.0 {
            fill(
                renderer,
                Rectangle {
                    width: elapsed_width,
                    ..rail
                },
                palette.primary.base.color,
                RAIL_HEIGHT / 2.0,
            );
        }

        if let Some(fraction) = state.hovered.filter(|_| self.is_live()) {
            let x = rail.x + rail.width * fraction;
            let ghost = Rectangle {
                x: x - 1.0,
                y: rail.y - 3.0,
                width: 2.0,
                height: rail.height + 6.0,
            };
            fill(
                renderer,
                ghost,
                palette.background.base.text.scale_alpha(0.5),
                1.0,
            );
        }

        if self.is_live() {
            let head = Rectangle {
                x: rail.x + elapsed_width - HEAD_RADIUS,
                y: rail.center_y() - HEAD_RADIUS,
                width: HEAD_RADIUS * 2.0,
                height: HEAD_RADIUS * 2.0,
            };
            fill(renderer, head, palette.primary.base.color, HEAD_RADIUS);
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if !self.is_live() {
            return mouse::Interaction::default();
        }

        let bounds = layout.bounds();
        if cursor.is_over(Self::seek_band(bounds)) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

fn fill(renderer: &mut Renderer, bounds: Rectangle, color: Color, radius: f32) {
    use iced::advanced::Renderer as _;

    renderer.fill_quad(
        Quad {
            bounds,
            border: Border {
                radius: radius.into(),
                ..Border::default()
            },
            ..Quad::default()
        },
        Background::Color(color),
    );
}

pub fn clock(seconds: f32) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "0:00".to_owned();
    }
    let total = seconds as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

impl<'a, Message: 'a> From<Timeline<'a, Message>> for Element<'a, Message, Theme, Renderer> {
    fn from(timeline: Timeline<'a, Message>) -> Self {
        Self::new(timeline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;

    fn bar(width: f32) -> Rectangle {
        Rectangle {
            x: 0.0,
            y: 0.0,
            width,
            height: MIN_HEIGHT,
        }
    }

    fn at(bounds: Rectangle, x: f32) -> mouse::Cursor {
        mouse::Cursor::Available(Point::new(x, bounds.center_y()))
    }

    fn timeline(position: f32, duration: f32) -> Timeline<'static, ()> {
        Timeline::new(position, duration, |_| ())
    }

    #[test]
    fn progress_is_the_played_share_of_the_track() {
        assert!((timeline(30.0, 120.0).progress() - 0.25).abs() < 0.001);
    }

    #[test]
    fn a_track_with_no_duration_never_divides_by_zero() {
        assert_eq!(timeline(10.0, 0.0).progress(), 0.0);
        assert!(!timeline(10.0, 0.0).is_live());
    }

    #[test]
    fn a_position_past_the_end_clamps_to_full() {
        assert_eq!(timeline(500.0, 120.0).progress(), 1.0);
    }

    #[test]
    fn the_rail_spans_the_whole_pane_width() {
        let bounds = bar(400.0);
        let rail = Timeline::<()>::rail(bounds);

        assert_eq!(rail.x, bounds.x);
        assert_eq!(rail.width, bounds.width);
    }

    #[test]
    fn a_pane_too_narrow_for_readouts_yields_no_negative_rail() {
        for width in [0.0, 20.0, 60.0, 96.0] {
            assert!(Timeline::<()>::rail(bar(width)).width >= 0.0, "{width}");
        }
    }

    #[test]
    fn the_playhead_fits_above_the_rail() {
        let bounds = bar(400.0);
        let rail = Timeline::<()>::rail(bounds);

        assert!(
            rail.center_y() - HEAD_RADIUS >= bounds.y,
            "the playhead was clipped by the top of the widget"
        );
    }

    #[test]
    fn the_seek_band_is_hittable_without_reaching_the_pane_s_rows() {
        let bounds = bar(400.0);
        let rail = Timeline::<()>::rail(bounds);
        let band = Timeline::<()>::seek_band(bounds);

        assert!(band.height > rail.height, "a 4px band would be unhittable");
        assert!(
            band.height >= HEAD_RADIUS * 2.0,
            "the band should at least cover the playhead the user is aiming at"
        );
        assert!(
            band.height <= bounds.height,
            "the widget is only the rail now, so its own height is the limit; \
             the rows above and below belong to the pane and are never in it"
        );
    }

    #[test]
    fn the_seek_band_never_escapes_the_widget() {
        for height in [0.0, 5.0, MIN_HEIGHT, 200.0] {
            let bounds = Rectangle {
                height,
                ..bar(400.0)
            };
            let band = Timeline::<()>::seek_band(bounds);

            assert!(band.height >= 0.0, "height {height} gave {}", band.height);
            assert!(
                band.y >= bounds.y,
                "height {height} started above the widget"
            );
            assert!(
                band.y + band.height <= bounds.y + bounds.height + 0.001,
                "height {height} ran past the widget"
            );
        }
    }

    #[test]
    fn clicking_the_far_left_seeks_to_the_start() {
        let bounds = bar(400.0);
        let rail = Timeline::<()>::rail(bounds);
        let fraction = Timeline::<()>::fraction_at(bounds, at(bounds, rail.x));

        assert_eq!(fraction, Some(0.0));
    }

    #[test]
    fn clicking_the_far_right_seeks_to_the_end() {
        let bounds = bar(400.0);
        let rail = Timeline::<()>::rail(bounds);
        let fraction = Timeline::<()>::fraction_at(bounds, at(bounds, rail.x + rail.width));

        assert_eq!(fraction, Some(1.0));
    }

    #[test]
    fn clicking_the_middle_seeks_to_the_middle() {
        let bounds = bar(400.0);
        let rail = Timeline::<()>::rail(bounds);
        let fraction =
            Timeline::<()>::fraction_at(bounds, at(bounds, rail.x + rail.width / 2.0)).unwrap();

        assert!((fraction - 0.5).abs() < 0.001, "{fraction}");
    }

    #[test]
    fn dragging_past_either_end_stays_in_range() {
        let bounds = bar(400.0);
        for x in [-500.0, -1.0, 401.0, 5_000.0] {
            let fraction = Timeline::<()>::fraction_at(bounds, at(bounds, x)).unwrap();
            assert!((0.0..=1.0).contains(&fraction), "x {x} gave {fraction}");
        }
    }

    #[test]
    fn clocks_read_as_minutes_and_seconds() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(9.0), "0:09");
        assert_eq!(clock(61.0), "1:01");
        assert_eq!(clock(3599.0), "59:59");
    }

    #[test]
    fn a_nonsense_position_still_reads_as_a_time() {
        assert_eq!(clock(f32::NAN), "0:00");
        assert_eq!(clock(-5.0), "0:00");
    }
}
