//! Renders a [`Layout`] tree into nested `row!`/`column!` widgets.
//!
//! This is the composition approach: rather than a custom layout widget, each
//! split becomes a flex row or column with one child sized by `Length::Fixed`
//! (a lock) or `Length::FillPortion` (a ratio). iced's own flex layout then
//! delivers the VSCode-sidebar rule for free — a fixed child holds its pixels
//! while fill children absorb window resize, and it compresses only when the
//! container underflows.
//!
//! In edit mode a thin draggable seam sits between a split's two children. It
//! captures a press and reports drags to the app, which resizes the split from
//! the cursor delta — no on-screen rectangle needed. The seam is layered over
//! the content with `stack` rather than inserted into the row/column, so it
//! never steals flex space from the panes.

use iced::widget::{Space, column, container, mouse_area, row, stack};
use iced::{Element, Length, mouse};

use crate::app::Message;
use crate::layout::{Axis, Layout, Node, PaneId, Side, Split, SplitPath};
use crate::pane::PaneKind;

const PORTION_SCALE: f32 = 1000.0;

const DIVIDER: f32 = 6.0;
const DIVIDER_LINE: f32 = 4.0;

pub fn view<'a>(
    layout: &'a Layout,
    edit_mode: bool,
    dragging: Option<Axis>,
    pane: &dyn Fn(PaneId, PaneKind, bool) -> Element<'a, Message>,
) -> Element<'a, Message> {
    let tree = render_node(&layout.root, &mut Vec::new(), layout, edit_mode, pane);

    match dragging {
        Some(axis) => mouse_area(tree).interaction(resize_cursor(axis)).into(),
        None => tree,
    }
}

fn resize_cursor(axis: Axis) -> mouse::Interaction {
    match axis {
        Axis::Vertical => mouse::Interaction::ResizingHorizontally,
        Axis::Horizontal => mouse::Interaction::ResizingVertically,
    }
}

fn render_node<'a>(
    node: &'a Node,
    path: &mut Vec<Side>,
    layout: &'a Layout,
    edit_mode: bool,
    pane: &dyn Fn(PaneId, PaneKind, bool) -> Element<'a, Message>,
) -> Element<'a, Message> {
    match node {
        Node::Leaf { id } => {
            let kind = layout.kind(*id).unwrap_or(PaneKind::Empty);
            pane(*id, kind, edit_mode)
        }
        Node::Split { axis, split, a, b } => {
            let here = SplitPath(path.clone());

            path.push(Side::A);
            let child_a = sized(
                render_node(a, path, layout, edit_mode, pane),
                *axis,
                side_length(*split, Side::A),
            );
            path.pop();

            path.push(Side::B);
            let child_b = sized(
                render_node(b, path, layout, edit_mode, pane),
                *axis,
                side_length(*split, Side::B),
            );
            path.pop();

            let content = match axis {
                Axis::Vertical => row![child_a, child_b].into(),
                Axis::Horizontal => column![child_a, child_b].into(),
            };

            if edit_mode {
                let seam = divider_overlay(here, *axis, *split);
                stack![content, seam].into()
            } else {
                content
            }
        }
    }
}

fn divider_overlay<'a>(path: SplitPath, axis: Axis, split: Split) -> Element<'a, Message> {
    let filler_a = filler(axis, shrink(side_length(split, Side::A), DIVIDER / 2.0));
    let filler_b = filler(axis, shrink(side_length(split, Side::B), DIVIDER / 2.0));
    let seam = divider(path, axis);

    match axis {
        Axis::Vertical => row![filler_a, seam, filler_b].into(),
        Axis::Horizontal => column![filler_a, seam, filler_b].into(),
    }
}

fn shrink(length: Length, by: f32) -> Length {
    match length {
        Length::Fixed(pixels) => Length::Fixed((pixels - by).max(0.0)),
        other => other,
    }
}

fn filler<'a>(axis: Axis, length: Length) -> Element<'a, Message> {
    match axis {
        Axis::Vertical => Space::new().width(length).height(Length::Fill).into(),
        Axis::Horizontal => Space::new().width(Length::Fill).height(length).into(),
    }
}

fn divider<'a>(path: SplitPath, axis: Axis) -> Element<'a, Message> {
    let (width, height) = match axis {
        Axis::Vertical => (Length::Fixed(DIVIDER), Length::Fill),
        Axis::Horizontal => (Length::Fill, Length::Fixed(DIVIDER)),
    };
    let (line_width, line_height) = match axis {
        Axis::Vertical => (Length::Fixed(DIVIDER_LINE), Length::Fill),
        Axis::Horizontal => (Length::Fill, Length::Fixed(DIVIDER_LINE)),
    };

    let line = container(Space::new())
        .width(line_width)
        .height(line_height)
        .style(crate::styles::divider_style);

    mouse_area(
        container(line)
            .width(width)
            .height(height)
            .center_x(width)
            .center_y(height),
    )
    .interaction(resize_cursor(axis))
    .on_press(Message::DividerGrabbed(path))
    .into()
}

fn side_length(split: Split, side: Side) -> Length {
    match split {
        Split::Locked {
            side: locked,
            pixels,
        } if locked == side => Length::Fixed(pixels),
        Split::Locked { .. } => Length::Fill,
        Split::Ratio { ratio } => {
            let portion = match side {
                Side::A => ratio * PORTION_SCALE,
                Side::B => (1.0 - ratio) * PORTION_SCALE,
            };
            Length::FillPortion(portion as u16)
        }
    }
}

fn sized(element: Element<'_, Message>, axis: Axis, length: Length) -> Element<'_, Message> {
    match axis {
        Axis::Vertical => container(element).width(length).height(Length::Fill).into(),
        Axis::Horizontal => container(element).width(Length::Fill).height(length).into(),
    }
}
