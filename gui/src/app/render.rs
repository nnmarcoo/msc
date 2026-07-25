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
//!
//! Two things keep the seam aligned with the boundary it represents, and both
//! are load-bearing. It is positioned by a single pixel-sized leading filler
//! rather than by sharing a flex row with the panes, since a fixed-width seam
//! in that row would shrink the pool the proportional fillers divide. And its
//! boundary is derived from the same truncated `FillPortion` values handed to
//! the flex layout, never from `ratio` directly — `FillPortion` takes a u16, so
//! the panes' real boundary is quantised, and computing the seam from the exact
//! ratio leaves the two disagreeing by a fraction of a pixel that shifts as the
//! ratio changes.
//!
//! The seam also reports its own split's extent when grabbed. Converting cursor
//! pixels to a ratio against the window's span instead makes a nested divider
//! lag the cursor and accumulate drift.

use iced::widget::{Space, column, container, mouse_area, responsive, row, stack};
use iced::{Element, Length, mouse};

use crate::app::Message;
use crate::layout::{Axis, Layout, Node, PaneId, Side, Split, SplitPath};
use crate::pane::PaneKind;

const PORTION_SCALE: f32 = 1000.0;

const DIVIDER: f32 = 6.0;
const DIVIDER_LINE: f32 = 3.0;

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
    responsive(move |size| {
        let span = match axis {
            Axis::Vertical => size.width,
            Axis::Horizontal => size.height,
        };

        let lead =
            (leading_extent(split, span) - DIVIDER / 2.0).clamp(0.0, (span - DIVIDER).max(0.0));

        let filler_a = filler(axis, Length::Fixed(lead));
        let seam = divider(path.clone(), axis, span);

        match axis {
            Axis::Vertical => row![filler_a, seam].into(),
            Axis::Horizontal => column![filler_a, seam].into(),
        }
    })
    .into()
}

fn leading_extent(split: Split, span: f32) -> f32 {
    match split {
        Split::Locked {
            side: Side::A,
            pixels,
        } => pixels,
        Split::Locked {
            side: Side::B,
            pixels,
        } => span - pixels,
        Split::Ratio { .. } => {
            let a = f32::from(portion(split, Side::A));
            let total = a + f32::from(portion(split, Side::B));
            if total == 0.0 {
                span / 2.0
            } else {
                span * a / total
            }
        }
    }
}

fn portion(split: Split, side: Side) -> u16 {
    match side_length(split, side) {
        Length::FillPortion(portion) => portion,
        _ => 0,
    }
}

fn filler<'a>(axis: Axis, length: Length) -> Element<'a, Message> {
    match axis {
        Axis::Vertical => Space::new().width(length).height(Length::Fill).into(),
        Axis::Horizontal => Space::new().width(Length::Fill).height(length).into(),
    }
}

fn divider<'a>(path: SplitPath, axis: Axis, span: f32) -> Element<'a, Message> {
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
    .on_press(Message::DividerGrabbed(path, span))
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

#[cfg(test)]
mod tests {
    use super::*;

    const SPAN: f32 = 1200.0;

    fn seam_center(split: Split, span: f32) -> f32 {
        let lead =
            (leading_extent(split, span) - DIVIDER / 2.0).clamp(0.0, (span - DIVIDER).max(0.0));
        lead + DIVIDER / 2.0
    }

    fn pane_boundary(split: Split, span: f32) -> f32 {
        let a = f32::from(portion(split, Side::A));
        let total = a + f32::from(portion(split, Side::B));
        span * a / total
    }

    #[test]
    fn seam_tracks_boundary_across_ratios() {
        for ratio in [0.05, 0.2, 0.35, 0.5, 0.65, 0.8, 0.95] {
            let split = Split::Ratio { ratio };
            let boundary = pane_boundary(split, SPAN);
            let seam = seam_center(split, SPAN);
            assert!(
                (seam - boundary).abs() < 0.01,
                "ratio {ratio}: seam {seam} vs boundary {boundary}"
            );
        }
    }

    #[test]
    fn seam_matches_panes_at_awkward_ratios() {
        for ratio in [
            0.4998,
            0.500_781_24,
            0.3337,
            0.6663,
            0.736_979_07,
            0.123_456,
        ] {
            let split = Split::Ratio { ratio };
            let boundary = pane_boundary(split, SPAN);
            let seam = seam_center(split, SPAN);
            assert!(
                (seam - boundary).abs() < 0.01,
                "ratio {ratio}: seam {seam} vs pane boundary {boundary}"
            );
        }
    }

    #[test]
    fn seam_tracks_boundary_when_side_a_locked() {
        let split = Split::Locked {
            side: Side::A,
            pixels: 240.0,
        };
        assert!((seam_center(split, SPAN) - 240.0).abs() < 0.01);
    }

    #[test]
    fn seam_tracks_boundary_when_side_b_locked() {
        let split = Split::Locked {
            side: Side::B,
            pixels: 240.0,
        };
        assert!((seam_center(split, SPAN) - (SPAN - 240.0)).abs() < 0.01);
    }

    #[test]
    fn seam_stays_inside_span_at_extremes() {
        for ratio in [0.0, 1.0] {
            let split = Split::Ratio { ratio };
            let lead =
                (leading_extent(split, SPAN) - DIVIDER / 2.0).clamp(0.0, (SPAN - DIVIDER).max(0.0));
            assert!(lead >= 0.0 && lead + DIVIDER <= SPAN, "ratio {ratio}");
        }
    }

    #[test]
    fn seam_handles_span_smaller_than_divider() {
        let split = Split::Ratio { ratio: 0.5 };
        let span = 4.0;
        let lead =
            (leading_extent(split, span) - DIVIDER / 2.0).clamp(0.0, (span - DIVIDER).max(0.0));
        assert!(lead.is_finite() && lead >= 0.0);
    }
}
