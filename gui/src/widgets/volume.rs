//! The volume slider: a draggable rail with a filled portion.
//!
//! The rail spans 0 to [`verse_core::VOLUME_MAX`], so the far end is a boost
//! above the level a file was mastered at rather than the mastered level itself.
//!
//! A custom widget rather than iced's `slider` for the same reason the timeline
//! is one: a 4px rail is not something anyone can hit, and a slider ties the hit
//! target to the rail's thickness. So the rail takes a band [`REACH`] taller
//! than itself on each side, and the widget is exactly that band. The readout
//! and the mute button beside it belong to [`crate::pane::volume`].
//!
//! Unlike the timeline this reports no hover position, and `Op::Set` applies on
//! every pointer move rather than on release. Seeking defers because kira
//! restarts the stream at each seek and doing that per-move is an audible
//! stutter; setting a volume is a tween on the playing sound, so a continuous
//! drag is the smooth ramp it looks like and needs no preview of where it would
//! land. `Op::Committed` fires once at the end, for the app to persist on.
//!
//! A press captures the pointer, so a drag that wanders off the rail keeps
//! setting the level and releasing anywhere ends it. That is why `dragging` is
//! widget state rather than being inferred from the cursor being over the rail.
//!
//! The widget never keeps its own copy of the level, so a level set here and one
//! set from a second volume pane cannot drift apart. Muting is likewise not its
//! concern: a muted player is drawn by handing the rail a zero, so the slider
//! shows the level coming out of the speakers and nothing else.

use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout};
use iced::{
    Background, Border, Color, Element, Event, Length, Rectangle, Renderer, Size, Theme, mouse,
};

const RAIL_HEIGHT: f32 = 4.0;
const HEAD_RADIUS: f32 = 5.0;
const REACH: f32 = 5.0;
const MIN_HEIGHT: f32 = RAIL_HEIGHT + REACH * 2.0;
const MIN_WIDTH: f32 = 40.0;

#[derive(Default)]
struct State {
    dragging: bool,
    hovered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Set(f32),
    Committed,
}

pub struct Volume<'a, Message> {
    level: f32,
    on_op: Box<dyn Fn(Op) -> Message + 'a>,
}

impl<'a, Message> Volume<'a, Message> {
    pub fn new(level: f32, on_op: impl Fn(Op) -> Message + 'a) -> Self {
        Self {
            level: level.clamp(0.0, verse_core::VOLUME_MAX),
            on_op: Box::new(on_op),
        }
    }

    pub const fn min_width() -> f32 {
        MIN_WIDTH
    }

    pub const fn height() -> f32 {
        MIN_HEIGHT
    }

    fn rail(bounds: Rectangle) -> Rectangle {
        Rectangle {
            x: bounds.x,
            y: bounds.center_y() - RAIL_HEIGHT / 2.0,
            width: bounds.width.max(0.0),
            height: RAIL_HEIGHT,
        }
    }

    fn level_at(bounds: Rectangle, cursor: mouse::Cursor) -> Option<f32> {
        let position = cursor.position()?;
        let rail = Self::rail(bounds);
        if rail.width <= 0.0 {
            return None;
        }
        let fraction = ((position.x - rail.x) / rail.width).clamp(0.0, 1.0);
        Some(fraction * verse_core::VOLUME_MAX)
    }

    fn fraction_of(level: f32) -> f32 {
        (level / verse_core::VOLUME_MAX).clamp(0.0, 1.0)
    }

    fn band(bounds: Rectangle) -> Rectangle {
        let rail = Self::rail(bounds);
        let top = (rail.y - REACH).max(bounds.y);
        let bottom = (rail.y + rail.height + REACH).min(bounds.y + bounds.height);
        Rectangle {
            x: bounds.x,
            y: top,
            width: bounds.width,
            height: (bottom - top).max(0.0),
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Volume<'_, Message> {
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

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(Self::band(bounds)) =>
            {
                if let Some(level) = Self::level_at(bounds, cursor) {
                    state.dragging = true;
                    shell.publish((self.on_op)(Op::Set(level)));
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.dragging {
                    if let Some(level) = Self::level_at(bounds, cursor) {
                        shell.publish((self.on_op)(Op::Set(level)));
                        shell.request_redraw();
                    }
                    return;
                }

                let over = cursor.is_over(Self::band(bounds));
                if over != state.hovered {
                    state.hovered = over;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.dragging => {
                state.dragging = false;
                shell.publish((self.on_op)(Op::Committed));
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta })
                if cursor.is_over(Self::band(bounds)) =>
            {
                let steps = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => y / PIXELS_PER_LINE,
                };
                if steps != 0.0 {
                    let level =
                        (self.level + steps * WHEEL_STEP).clamp(0.0, verse_core::VOLUME_MAX);
                    shell.publish((self.on_op)(Op::Set(level)));
                    shell.publish((self.on_op)(Op::Committed));
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorLeft) if !state.dragging && state.hovered => {
                state.hovered = false;
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

        let filled_width = rail.width * Self::fraction_of(self.level);

        fill(
            renderer,
            rail,
            palette.background.strong.color,
            RAIL_HEIGHT / 2.0,
        );

        if filled_width > 0.0 {
            fill(
                renderer,
                Rectangle {
                    width: filled_width,
                    ..rail
                },
                palette.primary.base.color,
                RAIL_HEIGHT / 2.0,
            );
        }

        if state.hovered || state.dragging {
            let head = Rectangle {
                x: rail.x + filled_width - HEAD_RADIUS,
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
        if cursor.is_over(Self::band(layout.bounds())) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

const WHEEL_STEP: f32 = 0.05;
const PIXELS_PER_LINE: f32 = 60.0;

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

pub fn percent(level: f32) -> String {
    if !level.is_finite() || level <= 0.0 {
        return "0%".to_owned();
    }
    let clamped = level.min(verse_core::VOLUME_MAX);
    format!("{}%", (clamped * 100.0).round() as u16)
}

impl<'a, Message: 'a> From<Volume<'a, Message>> for Element<'a, Message, Theme, Renderer> {
    fn from(volume: Volume<'a, Message>) -> Self {
        Self::new(volume)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
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

    #[test]
    fn a_level_outside_the_range_is_clamped_on_the_way_in() {
        let loud: Volume<'_, ()> = Volume::new(9.0, |_| ());
        let quiet: Volume<'_, ()> = Volume::new(-1.0, |_| ());

        assert_eq!(loud.level, verse_core::VOLUME_MAX);
        assert_eq!(quiet.level, 0.0);
    }

    #[test]
    fn a_boosted_level_is_kept_rather_than_clamped_to_unity() {
        let boosted: Volume<'_, ()> = Volume::new(1.5, |_| ());
        assert_eq!(boosted.level, 1.5, "boost should survive construction");
    }

    #[test]
    fn the_rail_spans_the_whole_pane_width() {
        let bounds = bar(200.0);
        let rail = Volume::<()>::rail(bounds);

        assert_eq!(rail.x, bounds.x);
        assert_eq!(rail.width, bounds.width);
    }

    #[test]
    fn a_pane_too_narrow_for_a_rail_yields_no_negative_width() {
        for width in [0.0, 10.0, MIN_WIDTH] {
            assert!(Volume::<()>::rail(bar(width)).width >= 0.0, "{width}");
        }
    }

    #[test]
    fn clicking_the_far_left_silences() {
        let bounds = bar(200.0);
        assert_eq!(Volume::<()>::level_at(bounds, at(bounds, 0.0)), Some(0.0));
    }

    #[test]
    fn clicking_the_far_right_is_the_loudest_boost() {
        let bounds = bar(200.0);
        assert_eq!(
            Volume::<()>::level_at(bounds, at(bounds, 200.0)),
            Some(verse_core::VOLUME_MAX)
        );
    }

    #[test]
    fn the_middle_of_the_rail_is_unity() {
        let bounds = bar(200.0);
        let level = Volume::<()>::level_at(bounds, at(bounds, 100.0)).unwrap();

        assert!(
            (level - 1.0).abs() < 0.001,
            "the rail's midpoint should be the mastered level, got {level}"
        );
    }

    #[test]
    fn the_upper_half_of_the_rail_boosts() {
        let bounds = bar(200.0);
        let level = Volume::<()>::level_at(bounds, at(bounds, 150.0)).unwrap();

        assert!(level > 1.0, "past the tick should boost, got {level}");
        assert!((level - 1.5).abs() < 0.001, "{level}");
    }

    #[test]
    fn dragging_past_either_end_stays_in_range() {
        let bounds = bar(200.0);
        for x in [-800.0, -1.0, 201.0, 5_000.0] {
            let level = Volume::<()>::level_at(bounds, at(bounds, x)).unwrap();
            assert!(
                (0.0..=verse_core::VOLUME_MAX).contains(&level),
                "x {x} gave {level}"
            );
        }
    }

    #[test]
    fn a_level_is_drawn_at_its_share_of_the_full_range() {
        assert!((Volume::<()>::fraction_of(1.0) - 0.5).abs() < 0.001);
        assert!((Volume::<()>::fraction_of(verse_core::VOLUME_MAX) - 1.0).abs() < 0.001);
    }

    #[test]
    fn the_band_is_hittable_without_escaping_the_widget() {
        let bounds = bar(200.0);
        let rail = Volume::<()>::rail(bounds);
        let band = Volume::<()>::band(bounds);

        assert!(band.height > rail.height, "a 4px band would be unhittable");
        assert!(
            band.height >= HEAD_RADIUS * 2.0,
            "the band should cover the head the user is aiming at"
        );
        assert!(band.height <= bounds.height);
    }

    #[test]
    fn the_band_never_escapes_a_widget_of_any_height() {
        for height in [0.0, 5.0, MIN_HEIGHT, 200.0] {
            let bounds = Rectangle {
                height,
                ..bar(200.0)
            };
            let band = Volume::<()>::band(bounds);

            assert!(band.height >= 0.0, "height {height} gave {}", band.height);
            assert!(band.y >= bounds.y, "height {height} started above");
            assert!(
                band.y + band.height <= bounds.y + bounds.height + 0.001,
                "height {height} ran past the widget"
            );
        }
    }

    #[test]
    fn the_head_fits_within_the_widget() {
        let bounds = bar(200.0);
        let rail = Volume::<()>::rail(bounds);

        assert!(
            rail.center_y() - HEAD_RADIUS >= bounds.y,
            "the head was clipped by the top of the widget"
        );
    }

    #[test]
    fn the_readout_is_a_whole_percentage() {
        assert_eq!(percent(0.0), "0%");
        assert_eq!(percent(0.5), "50%");
        assert_eq!(percent(1.0), "100%");
    }

    #[test]
    fn a_boosted_level_reads_above_one_hundred() {
        assert_eq!(percent(1.5), "150%");
        assert_eq!(percent(verse_core::VOLUME_MAX), "200%");
    }

    #[test]
    fn a_nonsense_level_still_reads_as_a_percentage() {
        assert_eq!(percent(f32::NAN), "0%");
        assert_eq!(percent(-3.0), "0%");
        assert_eq!(percent(99.0), "200%");
    }
}
