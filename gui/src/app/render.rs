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
//! the cursor delta — no on-screen rectangle needed.

use iced::widget::{column, container, mouse_area, row};
use iced::{Element, Length, mouse};

use crate::app::Message;
use crate::layout::{Axis, Layout, Node, PaneId, Side, Split, SplitPath};
use crate::pane::PaneKind;

const PORTION_SCALE: f32 = 1000.0;

const DIVIDER: f32 = 6.0;

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

            let divider = edit_mode.then(|| divider(here, *axis));

            match axis {
                Axis::Vertical => match divider {
                    Some(d) => row![child_a, d, child_b].into(),
                    None => row![child_a, child_b].into(),
                },
                Axis::Horizontal => match divider {
                    Some(d) => column![child_a, d, child_b].into(),
                    None => column![child_a, child_b].into(),
                },
            }
        }
    }
}

fn divider<'a>(path: SplitPath, axis: Axis) -> Element<'a, Message> {
    let (width, height) = match axis {
        Axis::Vertical => (Length::Fixed(DIVIDER), Length::Fill),
        Axis::Horizontal => (Length::Fill, Length::Fixed(DIVIDER)),
    };

    mouse_area(
        container(iced::widget::Space::new())
            .width(width)
            .height(height)
            .style(crate::styles::divider_style),
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
