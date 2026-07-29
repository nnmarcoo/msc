//! A tooltip that picks its own side.
//!
//! `tip(control, label)` is what every pane wants from a tooltip — the label, the
//! delay, the styling — without naming a side. A side named at the call site is a
//! constant standing in for a measurement, and it is wrong the moment the pane is
//! not the shape its author pictured: [`crate::pane::controls`] turns its buttons
//! into a column in a tall pane, and a `Position::Top` chosen for a row then draws
//! one button's label over the button above it.
//!
//! The caller cannot fix that, because it knows where its button sits in its own
//! row but not where that row sits in the window. Only the widget knows both, and
//! only once laid out.
//!
//! # How the side is chosen
//!
//! [`Placement::pick`] measures the gap to each edge of the viewport and takes the
//! first side in `ORDER` with room for the label, falling back to the roomiest
//! when none has room. The order is a preference rather than a ranking by space:
//! a label that jumped to whichever side was widest would move on every resize, so
//! a side with enough room keeps it even when another has more.
//!
//! Preference runs top, bottom, right, left. Above is where a label is looked for
//! and where it covers least, controls sitting nearer the bottom of a window than
//! the top.
//!
//! This reasons about the viewport, not about siblings. What saves the column case
//! is that a stack of buttons is tall and narrow, leaving the horizontal sides
//! free — a consequence of the geometry rather than a rule about columns, which is
//! why nothing here mentions them. A tooltip can still cover a neighbour in a
//! layout that puts one where the viewport looks empty.
//!
//! # Why it draws the label itself
//!
//! iced's tooltip fixes its side at construction, so there is no seam to hand a
//! measured side to: by the time the anchor's bounds are known the side is already
//! built in, and wrapping it would mean rebuilding the whole tooltip every frame
//! to change one field. [`Label`] is therefore a small overlay of its own, and
//! [`State`] keeps the hover clock iced would otherwise have kept.
//!
//! That overlay only draws. It answers `Interaction::None` and handles no events,
//! which is what keeps it clear of the trap in [`crate::widgets::context_menu`]:
//! an overlay taking input must claim the cursor over its ancestors or hover leaks
//! through, while one that only draws has nothing to claim.
//!
//! `hovered_at` is when the cursor arrived, and `settle` returns the moment the
//! label is due so [`Widget::update`] can ask for a redraw then — a pointer held
//! still produces no events, and without that the label would wait for a move that
//! never comes.

use std::time::Instant;

use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::widget::tooltip::Position;
use iced::widget::{container, text};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

use crate::styles::{self, LABEL_FONT_SIZE, PAD, TOOLTIP_DELAY};

pub fn tip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    label: &'a str,
) -> Tip<'a, Message> {
    Tip {
        content: content.into(),
        label: label_element(label),
        pinned: None,
    }
}

pub struct Tip<'a, Message> {
    content: Element<'a, Message>,
    label: Element<'a, Message>,
    pinned: Option<Position>,
}

impl<Message> Tip<'_, Message> {
    pub fn side(mut self, side: Position) -> Self {
        self.pinned = Some(side);
        self
    }
}

struct Placement;

impl Placement {
    const ORDER: [Position; 4] = [
        Position::Top,
        Position::Bottom,
        Position::Right,
        Position::Left,
    ];

    fn room(side: Position, anchor: Rectangle, viewport: Rectangle) -> f32 {
        let gap = match side {
            Position::Top => anchor.y - viewport.y,
            Position::Bottom => (viewport.y + viewport.height) - (anchor.y + anchor.height),
            Position::Left => anchor.x - viewport.x,
            Position::Right => (viewport.x + viewport.width) - (anchor.x + anchor.width),
            Position::FollowCursor => 0.0,
        };

        gap.max(0.0)
    }

    fn needed(side: Position, label: Size) -> f32 {
        let extent = match side {
            Position::Top | Position::Bottom => label.height,
            _ => label.width,
        };

        extent + PAD
    }

    fn pick(anchor: Rectangle, viewport: Rectangle, label: Size) -> Position {
        let spare = |side| Self::room(side, anchor, viewport) - Self::needed(side, label);
        let [first, rest @ ..] = Self::ORDER;

        if let Some(side) = Self::ORDER.into_iter().find(|&side| spare(side) >= 0.0) {
            return side;
        }

        rest.into_iter().fold(first, |best, side| {
            if spare(side) > spare(best) {
                side
            } else {
                best
            }
        })
    }
}

fn label_element<'a, Message: 'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label).size(LABEL_FONT_SIZE))
        .padding(PAD)
        .style(styles::tooltip_style)
        .into()
}

#[derive(Debug, Default, Clone, Copy)]
struct State {
    label: Size,
    hovered_at: Option<Instant>,
    open: bool,
}

impl State {
    fn settle(&mut self, hovering: bool, now: Instant) -> Option<Instant> {
        if !hovering {
            self.hovered_at = None;
            self.open = false;
            return None;
        }

        let since = *self.hovered_at.get_or_insert(now);
        let due = since + TOOLTIP_DELAY;

        if now >= due {
            self.open = true;
            None
        } else {
            Some(due)
        }
    }
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for Tip<'a, Message>
where
    Message: 'a,
{
    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        iced::advanced::widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content), Tree::new(&self.label)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.content.as_widget(), self.label.as_widget()]);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let measured = self.label.as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &layout::Limits::NONE,
        );

        tree.state.downcast_mut::<State>().label = measured.bounds().size();

        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let state = tree.state.downcast_mut::<State>();
        let was_open = state.open;

        match state.settle(cursor.is_over(layout.bounds()), Instant::now()) {
            Some(due) => shell.request_redraw_at(due),
            None if state.open != was_open => {
                shell.invalidate_layout();
                shell.request_redraw();
            }
            None => {}
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        let State { open, label, .. } = *tree.state.downcast_ref::<State>();
        let (content_tree, label_tree) = tree.children.split_at_mut(1);

        let beneath = self.content.as_widget_mut().overlay(
            &mut content_tree[0],
            layout,
            renderer,
            viewport,
            translation,
        );

        if beneath.is_some() || !open {
            return beneath;
        }

        let anchor = layout.bounds() + translation;

        Some(overlay::Element::new(Box::new(Label {
            element: &mut self.label,
            tree: &mut label_tree[0],
            anchor,
            side: self
                .pinned
                .unwrap_or_else(|| Placement::pick(anchor, *viewport, label)),
        })))
    }
}

struct Label<'a, 'b, Message> {
    element: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    anchor: Rectangle,
    side: Position,
}

impl<Message> overlay::Overlay<Message, iced::Theme, iced::Renderer> for Label<'_, '_, Message> {
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let node = self
            .element
            .as_widget_mut()
            .layout(self.tree, renderer, &layout::Limits::NONE);

        let label = node.bounds().size();
        let anchor = self.anchor;

        let centred_x = anchor.x + (anchor.width - label.width) / 2.0;
        let centred_y = anchor.y + (anchor.height - label.height) / 2.0;

        let position = match self.side {
            Position::Bottom => iced::Point::new(centred_x, anchor.y + anchor.height + PAD),
            Position::Left => iced::Point::new(anchor.x - label.width - PAD, centred_y),
            Position::Right => iced::Point::new(anchor.x + anchor.width + PAD, centred_y),
            Position::Top | Position::FollowCursor => {
                iced::Point::new(centred_x, anchor.y - label.height - PAD)
            }
        };

        node.move_to(iced::Point::new(
            position.x.clamp(0.0, (bounds.width - label.width).max(0.0)),
            position
                .y
                .clamp(0.0, (bounds.height - label.height).max(0.0)),
        ))
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.element.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn mouse_interaction(
        &self,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }
}

impl<'a, Message> From<Tip<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(tip: Tip<'a, Message>) -> Self {
        Element::new(tip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> Rectangle {
        Rectangle::new(iced::Point::new(0.0, 0.0), Size::new(800.0, 600.0))
    }

    fn label() -> Size {
        Size::new(60.0, 16.0)
    }

    fn anchor(x: f32, y: f32) -> Rectangle {
        Rectangle::new(iced::Point::new(x, y), Size::new(30.0, 30.0))
    }

    #[test]
    fn a_button_with_room_above_is_labelled_above() {
        assert_eq!(
            Placement::pick(anchor(400.0, 300.0), viewport(), label()),
            Position::Top
        );
    }

    #[test]
    fn a_button_against_the_top_is_labelled_below() {
        assert_eq!(
            Placement::pick(anchor(400.0, 0.0), viewport(), label()),
            Position::Bottom
        );
    }

    #[test]
    fn a_button_filling_the_height_is_labelled_beside() {
        let tall = Rectangle::new(iced::Point::new(400.0, 0.0), Size::new(30.0, 600.0));
        assert_eq!(Placement::pick(tall, viewport(), label()), Position::Right);
    }

    #[test]
    fn a_button_against_the_right_edge_is_labelled_left() {
        let tall = Rectangle::new(iced::Point::new(770.0, 0.0), Size::new(30.0, 600.0));
        assert_eq!(Placement::pick(tall, viewport(), label()), Position::Left);
    }

    #[test]
    fn the_side_holds_while_it_has_room() {
        let settled = Placement::pick(anchor(200.0, 300.0), viewport(), label());

        for width in [400.0, 600.0, 800.0, 1200.0] {
            let wider = Rectangle::new(iced::Point::new(0.0, 0.0), Size::new(width, 600.0));
            assert_eq!(
                Placement::pick(anchor(200.0, 300.0), wider, label()),
                settled,
                "the label moved at {width} though its side still had room"
            );
        }
    }

    #[test]
    fn a_label_with_room_nowhere_still_picks_a_side() {
        let cramped = Rectangle::new(iced::Point::new(0.0, 0.0), Size::new(60.0, 60.0));
        let anchor = Rectangle::new(iced::Point::new(25.0, 25.0), Size::new(10.0, 10.0));

        let side = Placement::pick(anchor, cramped, Size::new(500.0, 500.0));
        assert!(
            Placement::ORDER.contains(&side),
            "picked {side:?}, which is not a side it may choose"
        );
    }

    #[test]
    fn the_fallback_takes_the_side_with_the_most_room() {
        let viewport = Rectangle::new(iced::Point::new(0.0, 0.0), Size::new(400.0, 400.0));
        let anchor = Rectangle::new(iced::Point::new(200.0, 0.0), Size::new(10.0, 10.0));

        assert_eq!(
            Placement::pick(anchor, viewport, Size::new(1000.0, 1000.0)),
            Position::Bottom
        );
    }

    #[test]
    fn room_never_goes_negative() {
        let outside = Rectangle::new(iced::Point::new(-100.0, -100.0), Size::new(10.0, 10.0));

        for side in Placement::ORDER {
            assert!(Placement::room(side, outside, viewport()) >= 0.0);
        }
    }

    #[test]
    fn a_taller_label_needs_more_room_above_than_a_short_one() {
        let short = Placement::needed(Position::Top, Size::new(60.0, 16.0));
        let tall = Placement::needed(Position::Top, Size::new(60.0, 40.0));
        assert!(tall > short);
    }

    #[test]
    fn a_wider_label_needs_more_room_beside_than_a_narrow_one() {
        let narrow = Placement::needed(Position::Right, Size::new(40.0, 16.0));
        let wide = Placement::needed(Position::Right, Size::new(200.0, 16.0));
        assert!(wide > narrow);
    }

    #[test]
    fn a_label_waits_out_the_delay_before_it_opens() {
        let mut state = State::default();
        let start = Instant::now();

        let due = state
            .settle(true, start)
            .expect("the label is still waiting");
        assert!(!state.open);
        assert_eq!(due, start + TOOLTIP_DELAY);

        assert_eq!(state.settle(true, due), None);
        assert!(state.open, "the delay elapsed but the label stayed shut");
    }

    #[test]
    fn the_clock_runs_from_arrival_rather_than_the_latest_event() {
        let mut state = State::default();
        let start = Instant::now();

        state.settle(true, start);
        state.settle(true, start + TOOLTIP_DELAY / 2);
        state.settle(true, start + TOOLTIP_DELAY);

        assert!(
            state.open,
            "a pointer that moved within the button restarted the delay"
        );
    }

    #[test]
    fn leaving_shuts_the_label_and_forgets_the_clock() {
        let mut state = State::default();
        let start = Instant::now();

        state.settle(true, start);
        state.settle(true, start + TOOLTIP_DELAY);
        assert!(state.open);

        assert_eq!(state.settle(false, start + TOOLTIP_DELAY), None);
        assert!(!state.open);
        assert!(state.hovered_at.is_none());
    }

    #[test]
    fn returning_waits_the_full_delay_again() {
        let mut state = State::default();
        let start = Instant::now();

        state.settle(true, start);
        state.settle(false, start + TOOLTIP_DELAY);

        let again = start + TOOLTIP_DELAY * 2;
        assert_eq!(state.settle(true, again), Some(again + TOOLTIP_DELAY));
        assert!(!state.open, "the label reopened without a fresh wait");
    }
}
