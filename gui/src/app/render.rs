//! Renders a [`Layout`] tree into nested `row!`/`column!` widgets.
//!
//! This is the composition approach: rather than a custom layout widget, each
//! split becomes a flex row or column with one child sized by `Length::Fixed`
//! (a lock) or `Length::FillPortion` (a ratio). iced's own flex layout then
//! delivers the VSCode-sidebar rule for free. A fixed child holds its pixels
//! while fill children absorb window resize, and it compresses only when the
//! container underflows.
//!
//! In edit mode a thin draggable seam sits between a split's two children. It
//! captures a press and reports drags to the app, which resizes the split from
//! the cursor delta, with no on-screen rectangle needed. The seam is layered over
//! the content with `stack` rather than inserted into the row/column, so it
//! never steals flex space from the panes.
//!
//! Two things keep the seam aligned with the boundary it represents, and both
//! are load-bearing. It is positioned by a single pixel-sized leading filler
//! rather than by sharing a flex row with the panes, since a fixed-width seam
//! in that row would shrink the pool the proportional fillers divide. And its
//! boundary is derived from the same truncated `FillPortion` values handed to
//! the flex layout, never from `ratio` directly. `FillPortion` takes a u16, so
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
    pane: &dyn Fn(PaneId, PaneKind, bool, iced::Size) -> Element<'a, Message>,
    window: iced::Size,
) -> Element<'a, Message> {
    let tree = render_node(&layout.root, &mut Vec::new(), layout, edit_mode, pane, window);

    match dragging {
        Some(axis) => mouse_area(tree).interaction(resize_cursor(axis)).into(),
        None => tree,
    }
}

fn budget(a: Option<f32>, b: Option<f32>) -> Option<f32> {
    match (a, b) {
        (Some(a), Some(b)) if a + b > 0.0 => Some(a + b),
        _ => None,
    }
}

fn shrink_to_fit(
    content: Element<'_, Message>,
    axis: Axis,
    budget: Option<f32>,
) -> Element<'_, Message> {
    let Some(total) = budget else {
        return content;
    };

    let capped = match axis {
        Axis::Vertical => container(content).max_width(total),
        Axis::Horizontal => container(content).max_height(total),
    };
    capped.width(Length::Fill).height(Length::Fill).into()
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
    pane: &dyn Fn(PaneId, PaneKind, bool, iced::Size) -> Element<'a, Message>,
    span: iced::Size,
) -> Element<'a, Message> {
    match node {
        Node::Leaf { id } => {
            let kind = layout.kind(*id).unwrap_or(PaneKind::Empty);
            pane(*id, kind, edit_mode, span)
        }
        Node::Split { axis, split, a, b } => {
            let fixed_a = fixed_extent(a, *axis, layout);
            let fixed_b = fixed_extent(b, *axis, layout);

            let span_a = nested_span(a, fixed_a, fixed_b, *axis, *split, Side::A, span);
            let span_b = nested_span(b, fixed_b, fixed_a, *axis, *split, Side::B, span);

            path.push(Side::A);
            let child_a = render_node(a, path, layout, edit_mode, pane, span_a);
            path.pop();

            path.push(Side::B);
            let child_b = render_node(b, path, layout, edit_mode, pane, span_b);
            path.pop();

            let child_a = sized(child_a, *axis, child_length(*split, Side::A, fixed_a));
            let child_b = sized(child_b, *axis, child_length(*split, Side::B, fixed_b));

            let row: Element<'a, Message> = match axis {
                Axis::Vertical => row![child_a, child_b].into(),
                Axis::Horizontal => column![child_a, child_b].into(),
            };
            let content = shrink_to_fit(row, *axis, budget(fixed_a, fixed_b));

            if !edit_mode {
                return content;
            }

            let adjacent = adjacent_lock(a, *axis, layout).is_some()
                || adjacent_lock(b, *axis, layout).is_some();
            let inert = !adjacent && (fixed_a.is_some() || fixed_b.is_some());
            let lead = fixed_a
                .map(Lead::Fixed)
                .or_else(|| fixed_b.map(Lead::FromEnd));
            let seam = divider_overlay(
                SplitPath(path.clone()),
                *axis,
                *split,
                lead,
                inert,
                adjacent,
            );
            stack![content, seam].into()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Lead {
    Fixed(f32),
    FromEnd(f32),
}

impl Lead {
    fn resolve(self, span: f32) -> f32 {
        match self {
            Self::Fixed(pixels) => pixels,
            Self::FromEnd(pixels) => span - pixels,
        }
    }
}

fn adjacent_lock(child: &Node, axis: Axis, layout: &Layout) -> Option<f32> {
    match child {
        Node::Leaf { id } => layout.locks(*id).along(axis),
        Node::Split { .. } => None,
    }
}

fn divider_overlay<'a>(
    path: SplitPath,
    axis: Axis,
    split: Split,
    lead: Option<Lead>,
    inert: bool,
    locked: bool,
) -> Element<'a, Message> {
    responsive(move |size| {
        let span = match axis {
            Axis::Vertical => size.width,
            Axis::Horizontal => size.height,
        };

        let boundary = lead.map_or_else(
            || leading_extent(split, span),
            |lead| lead.resolve(span).clamp(0.0, span),
        );
        let lead = (boundary - DIVIDER / 2.0).clamp(0.0, (span - DIVIDER).max(0.0));

        let filler_a = filler(axis, Length::Fixed(lead));
        let seam = divider(path.clone(), axis, span, inert, locked);

        match axis {
            Axis::Vertical => row![filler_a, seam].into(),
            Axis::Horizontal => column![filler_a, seam].into(),
        }
    })
    .into()
}

fn leading_extent(split: Split, span: f32) -> f32 {
    let a = f32::from(portion(split, Side::A));
    let total = a + f32::from(portion(split, Side::B));
    if total == 0.0 {
        span / 2.0
    } else {
        span * a / total
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

fn divider<'a>(
    path: SplitPath,
    axis: Axis,
    span: f32,
    inert: bool,
    locked: bool,
) -> Element<'a, Message> {
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
        .style(if inert || locked {
            crate::styles::divider_locked_style
        } else {
            crate::styles::divider_style
        });

    let seam = container(line)
        .width(width)
        .height(height)
        .center_x(width)
        .center_y(height);

    if inert {
        return seam.into();
    }

    mouse_area(seam)
        .interaction(resize_cursor(axis))
        .on_press(Message::DividerGrabbed(path, span))
        .into()
}

fn nested_span(
    child: &Node,
    fixed: Option<f32>,
    sibling_fixed: Option<f32>,
    axis: Axis,
    split: Split,
    side: Side,
    span: iced::Size,
) -> iced::Size {
    if matches!(child, Node::Leaf { .. }) {
        return span;
    }

    let available = match axis {
        Axis::Vertical => span.width,
        Axis::Horizontal => span.height,
    };

    let extent = if let Some(pixels) = fixed {
        pixels.min(available)
    } else if let Some(taken) = sibling_fixed {
        (available - taken).max(0.0)
    } else {
        let share = match side {
            Side::A => split.ratio,
            Side::B => 1.0 - split.ratio,
        };
        available * share
    };

    match axis {
        Axis::Vertical => iced::Size::new(extent, span.height),
        Axis::Horizontal => iced::Size::new(span.width, extent),
    }
}

fn child_length(split: Split, side: Side, fixed: Option<f32>) -> Length {
    match fixed {
        Some(pixels) => Length::Fixed(pixels),
        None => side_length(split, side),
    }
}

fn fixed_extent(node: &Node, axis: Axis, layout: &Layout) -> Option<f32> {
    match node {
        Node::Leaf { id } => layout.locks(*id).along(axis),
        Node::Split {
            axis: inner, a, b, ..
        } => {
            let (a, b) = (fixed_extent(a, axis, layout), fixed_extent(b, axis, layout));
            if *inner == axis {
                Some(a? + b?)
            } else {
                match (a, b) {
                    (Some(a), Some(b)) => Some(a.max(b)),
                    (found, None) | (None, found) => found,
                }
            }
        }
    }
}

fn side_length(split: Split, side: Side) -> Length {
    let portion = match side {
        Side::A => split.ratio * PORTION_SCALE,
        Side::B => (1.0 - split.ratio) * PORTION_SCALE,
    };
    Length::FillPortion(portion as u16)
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
    use crate::layout::{Locks, PaneEntry};

    const SPAN: f32 = 1200.0;

    const AXIS: Axis = Axis::Vertical;

    fn leaf(id: u32) -> Node {
        Node::leaf(PaneId(id))
    }

    fn split_node(axis: Axis, split: Split, a: Node, b: Node) -> Node {
        Node::Split {
            axis,
            split,
            a: Box::new(a),
            b: Box::new(b),
        }
    }

    fn ratio(ratio: f32) -> Split {
        Split { ratio }
    }

    fn wide(pixels: f32) -> Locks {
        Locks {
            width: Some(pixels),
            height: None,
        }
    }

    fn tall(pixels: f32) -> Locks {
        Locks {
            width: None,
            height: Some(pixels),
        }
    }

    fn layout_of(axis: Axis, locks: [Locks; 2]) -> Layout {
        Layout {
            name: "t".into(),
            root: split_node(axis, Split::default(), leaf(0), leaf(1)),
            panes: locks
                .into_iter()
                .enumerate()
                .map(|(index, locks)| PaneEntry {
                    id: PaneId(index as u32),
                    kind: PaneKind::Empty,
                    locks,
                })
                .collect(),
        }
    }

    fn root_split(layout: &Layout) -> (Split, &Node, &Node) {
        match &layout.root {
            Node::Split { split, a, b, .. } => (*split, a, b),
            Node::Leaf { .. } => panic!("expected a split"),
        }
    }

    fn seam_center(layout: &Layout, span: f32) -> f32 {
        let (split, a, b) = root_split(layout);
        let lead = fixed_extent(a, AXIS, layout)
            .map(Lead::Fixed)
            .or_else(|| fixed_extent(b, AXIS, layout).map(Lead::FromEnd));
        let boundary = lead.map_or_else(
            || leading_extent(split, span),
            |lead| lead.resolve(span).clamp(0.0, span),
        );
        let clamped = (boundary - DIVIDER / 2.0).clamp(0.0, (span - DIVIDER).max(0.0));
        clamped + DIVIDER / 2.0
    }

    fn leaf_length(layout: &Layout, side: Side) -> Length {
        let (split, a, b) = root_split(layout);
        let child = match side {
            Side::A => a,
            Side::B => b,
        };
        child_length(split, side, fixed_extent(child, AXIS, layout))
    }

    fn pane_boundary(split: Split, span: f32) -> f32 {
        let a = f32::from(portion(split, Side::A));
        let total = a + f32::from(portion(split, Side::B));
        span * a / total
    }

    fn unlocked() -> [Locks; 2] {
        [Locks::default(), Locks::default()]
    }

    #[test]
    fn seam_tracks_boundary_across_ratios() {
        for r in [0.05, 0.2, 0.35, 0.5, 0.65, 0.8, 0.95] {
            let mut layout = layout_of(AXIS, unlocked());
            if let Node::Split { split, .. } = &mut layout.root {
                split.ratio = r;
            }
            let (split, ..) = root_split(&layout);
            let boundary = pane_boundary(split, SPAN);
            let seam = seam_center(&layout, SPAN);
            assert!(
                (seam - boundary).abs() < 0.01,
                "ratio {r}: seam {seam} vs boundary {boundary}"
            );
        }
    }

    #[test]
    fn seam_matches_panes_at_awkward_ratios() {
        for r in [
            0.4998,
            0.500_781_24,
            0.3337,
            0.6663,
            0.736_979_07,
            0.123_456,
        ] {
            let mut layout = layout_of(AXIS, unlocked());
            if let Node::Split { split, .. } = &mut layout.root {
                split.ratio = r;
            }
            let (split, ..) = root_split(&layout);
            let boundary = pane_boundary(split, SPAN);
            let seam = seam_center(&layout, SPAN);
            assert!(
                (seam - boundary).abs() < 0.01,
                "ratio {r}: seam {seam} vs pane boundary {boundary}"
            );
        }
    }

    #[test]
    fn a_width_locked_pane_holds_width_and_flexes_in_height() {
        let row = layout_of(Axis::Vertical, [Locks::default(), wide(320.0)]);
        assert_eq!(leaf_length(&row, Side::B), Length::Fixed(320.0));
        assert!(matches!(leaf_length(&row, Side::A), Length::FillPortion(_)));

        let (_, _, b) = root_split(&row);
        assert_eq!(
            fixed_extent(b, Axis::Horizontal, &row),
            None,
            "a width lock constrained the height"
        );
    }

    #[test]
    fn a_height_locked_pane_holds_height_and_flexes_in_width() {
        let column = layout_of(Axis::Horizontal, [Locks::default(), tall(120.0)]);
        let (_, _, b) = root_split(&column);
        assert_eq!(fixed_extent(b, Axis::Horizontal, &column), Some(120.0));
        assert_eq!(
            fixed_extent(b, Axis::Vertical, &column),
            None,
            "a height lock constrained the width"
        );
    }

    #[test]
    fn a_pane_can_lock_both_axes() {
        let both = Locks {
            width: Some(300.0),
            height: Some(110.0),
        };
        let row = layout_of(Axis::Vertical, [Locks::default(), both]);
        let (_, _, b) = root_split(&row);
        assert_eq!(fixed_extent(b, Axis::Vertical, &row), Some(300.0));
        assert_eq!(fixed_extent(b, Axis::Horizontal, &row), Some(110.0));
    }

    #[test]
    fn a_locked_pane_keeps_its_pixels_as_the_window_shrinks() {
        let row = layout_of(Axis::Vertical, [wide(240.0), Locks::default()]);
        for span in [1600.0, 1000.0, 600.0, 320.0] {
            assert_eq!(leaf_length(&row, Side::A), Length::Fixed(240.0));
            assert!((seam_center(&row, span) - 240.0).abs() < 0.01, "span {span}");
        }
    }

    #[test]
    fn an_unlocked_layout_is_fully_proportional() {
        let layout = layout_of(AXIS, unlocked());
        assert!(matches!(
            leaf_length(&layout, Side::A),
            Length::FillPortion(_)
        ));
        let (_, a, _) = root_split(&layout);
        assert_eq!(fixed_extent(a, AXIS, &layout), None);
    }

    #[test]
    fn locks_propagate_up_through_a_subtree() {
        let layout = Layout {
            name: "t".into(),
            root: split_node(
                Axis::Horizontal,
                Split::default(),
                leaf(0),
                split_node(Axis::Vertical, Split::default(), leaf(1), leaf(2)),
            ),
            panes: vec![
                PaneEntry {
                    id: PaneId(0),
                    kind: PaneKind::Empty,
                    locks: Locks::default(),
                },
                PaneEntry {
                    id: PaneId(1),
                    kind: PaneKind::Empty,
                    locks: tall(150.0),
                },
                PaneEntry {
                    id: PaneId(2),
                    kind: PaneKind::Empty,
                    locks: tall(150.0),
                },
            ],
        };

        let (_, _, bottom) = root_split(&layout);
        assert_eq!(
            fixed_extent(bottom, Axis::Horizontal, &layout),
            Some(150.0),
            "bottom row did not hold its height"
        );
        assert_eq!(
            fixed_extent(bottom, Axis::Vertical, &layout),
            None,
            "bottom row pinned its width too"
        );
    }

    #[test]
    fn side_by_side_widths_sum() {
        let row = layout_of(Axis::Vertical, [wide(200.0), wide(300.0)]);
        assert_eq!(fixed_extent(&row.root, Axis::Vertical, &row), Some(500.0));
    }

    #[test]
    fn stacked_heights_sum() {
        let column = layout_of(Axis::Horizontal, [tall(120.0), tall(180.0)]);
        assert_eq!(
            fixed_extent(&column.root, Axis::Horizontal, &column),
            Some(300.0)
        );
    }

    #[test]
    fn across_the_axis_the_larger_demand_wins() {
        let row = layout_of(Axis::Vertical, [tall(120.0), tall(180.0)]);
        assert_eq!(fixed_extent(&row.root, Axis::Horizontal, &row), Some(180.0));
    }

    #[test]
    fn one_unlocked_side_leaves_the_split_elastic() {
        let row = layout_of(Axis::Vertical, [wide(200.0), Locks::default()]);
        assert_eq!(fixed_extent(&row.root, Axis::Vertical, &row), None);
    }

    #[test]
    fn adjacent_locks_keep_a_seam_draggable() {
        let row = layout_of(Axis::Vertical, [wide(240.0), Locks::default()]);
        let (_, a, b) = root_split(&row);
        assert!(
            adjacent_lock(a, Axis::Vertical, &row).is_some()
                || adjacent_lock(b, Axis::Vertical, &row).is_some()
        );
    }

    #[test]
    fn a_subtree_lock_does_not_make_a_seam_draggable() {
        let subtree = split_node(Axis::Vertical, Split::default(), leaf(1), leaf(2));
        let layout = Layout {
            name: "t".into(),
            root: split_node(Axis::Horizontal, Split::default(), leaf(0), subtree),
            panes: vec![
                PaneEntry {
                    id: PaneId(0),
                    kind: PaneKind::Empty,
                    locks: Locks::default(),
                },
                PaneEntry {
                    id: PaneId(1),
                    kind: PaneKind::Empty,
                    locks: tall(150.0),
                },
                PaneEntry {
                    id: PaneId(2),
                    kind: PaneKind::Empty,
                    locks: tall(150.0),
                },
            ],
        };
        let (_, _, bottom) = root_split(&layout);
        assert_eq!(adjacent_lock(bottom, Axis::Horizontal, &layout), None);
    }

    #[test]
    fn a_split_with_a_free_side_needs_no_budget() {
        let row = layout_of(Axis::Vertical, [wide(240.0), Locks::default()]);
        let (_, a, b) = root_split(&row);
        assert_eq!(budget(fixed_extent(a, Axis::Vertical, &row), fixed_extent(b, Axis::Vertical, &row)), None);
    }

    #[test]
    fn a_fully_locked_split_is_capped_at_its_total() {
        let row = layout_of(Axis::Vertical, [wide(200.0), wide(300.0)]);
        let (_, a, b) = root_split(&row);
        let budget = budget(fixed_extent(a, Axis::Vertical, &row), fixed_extent(b, Axis::Vertical, &row)).expect("both sides locked");
        assert!((budget - 500.0).abs() < 0.01, "{}", budget);
    }

    #[test]
    fn a_budget_only_covers_its_own_axis() {
        let row = layout_of(Axis::Vertical, [wide(200.0), wide(300.0)]);
        let (_, a, b) = root_split(&row);
        assert!(budget(fixed_extent(a, Axis::Vertical, &row), fixed_extent(b, Axis::Vertical, &row)).is_some());
        assert_eq!(budget(fixed_extent(a, Axis::Horizontal, &row), fixed_extent(b, Axis::Horizontal, &row)), None);
    }

    #[test]
    fn a_leaf_is_told_its_parents_span_not_its_own() {
        let row = layout_of(Axis::Vertical, unlocked());
        let (split, a, b) = root_split(&row);
        let window = iced::Size::new(1000.0, 600.0);

        let span = nested_span(a, fixed_extent(a, Axis::Vertical, &row), fixed_extent(b, Axis::Vertical, &row), Axis::Vertical, split, Side::A, window);
        assert!(
            (span.width - 1000.0).abs() < 0.01,
            "leaf got {} as its span; expected the split's full 1000",
            span.width
        );
    }

    #[test]
    fn a_nested_split_narrows_only_its_own_axis() {
        let layout = layout_of(Axis::Vertical, unlocked());
        let inner = split_node(Axis::Horizontal, Split::default(), leaf(1), leaf(2));
        let outer = Split { ratio: 0.4 };
        let window = iced::Size::new(1000.0, 600.0);

        let span = nested_span(&inner, fixed_extent(&inner, Axis::Vertical, &layout), fixed_extent(&leaf(0), Axis::Vertical, &layout), Axis::Vertical, outer, Side::B, window);
        assert!(
            (span.width - 600.0).abs() < 0.01,
            "width narrowed to {} instead of the 0.6 share",
            span.width
        );
        assert!(
            (span.height - 600.0).abs() < 0.01,
            "height should pass through untouched, got {}",
            span.height
        );
    }

    #[test]
    fn a_nested_leaf_gets_both_governing_spans() {
        let layout = layout_of(Axis::Vertical, unlocked());
        let window = iced::Size::new(1000.0, 600.0);
        let outer = Split { ratio: 0.6 };
        let inner_node = split_node(Axis::Horizontal, Split::default(), leaf(1), leaf(2));

        let inner_span =
            nested_span(&inner_node, fixed_extent(&inner_node, Axis::Vertical, &layout), fixed_extent(&leaf(0), Axis::Vertical, &layout), Axis::Vertical, outer, Side::B, window);
        assert!((inner_span.width - 400.0).abs() < 0.01, "{}", inner_span.width);
        assert!((inner_span.height - 600.0).abs() < 0.01, "{}", inner_span.height);

        let leaf_span = nested_span(
            &leaf(1),
            fixed_extent(&leaf(1), Axis::Horizontal, &layout),
            fixed_extent(&leaf(2), Axis::Horizontal, &layout),
            Axis::Horizontal,
            Split::default(),
            Side::A,
            inner_span,
        );
        assert!(
            (leaf_span.width - 400.0).abs() < 0.01,
            "leaf width span {} should match the split it sits in",
            leaf_span.width
        );
        assert!(
            (leaf_span.height - 600.0).abs() < 0.01,
            "leaf height span {} should match the split it sits in",
            leaf_span.height
        );
    }

    #[test]
    fn a_locked_leaf_measures_against_its_whole_split() {
        let row = layout_of(Axis::Vertical, [wide(320.0), Locks::default()]);
        let (split, a, b) = root_split(&row);
        let window = iced::Size::new(1000.0, 600.0);

        let span = nested_span(a, fixed_extent(a, Axis::Vertical, &row), fixed_extent(b, Axis::Vertical, &row), Axis::Vertical, split, Side::A, window);
        assert!(
            (span.width - 1000.0).abs() < 0.01,
            "locked leaf got {} as its span; a share against it would be {:.2}, not 0.32",
            span.width,
            320.0 / span.width
        );
    }

    #[test]
    fn a_locked_subtree_span_follows_the_lock_not_the_ratio() {
        let layout = Layout {
            name: "t".into(),
            root: split_node(
                Axis::Vertical,
                Split { ratio: 0.5 },
                leaf(0),
                split_node(Axis::Horizontal, Split::default(), leaf(1), leaf(2)),
            ),
            panes: vec![
                PaneEntry {
                    id: PaneId(0),
                    kind: PaneKind::Empty,
                    locks: Locks::default(),
                },
                PaneEntry {
                    id: PaneId(1),
                    kind: PaneKind::Empty,
                    locks: wide(320.0),
                },
                PaneEntry {
                    id: PaneId(2),
                    kind: PaneKind::Empty,
                    locks: wide(320.0),
                },
            ],
        };

        let (split, a, b) = root_split(&layout);
        let window = iced::Size::new(1000.0, 600.0);
        let span = nested_span(b, fixed_extent(b, Axis::Vertical, &layout), fixed_extent(a, Axis::Vertical, &layout), Axis::Vertical, split, Side::B, window);

        assert!(
            (span.width - 320.0).abs() < 0.01,
            "locked subtree got {} as its span; the ratio would have said 500",
            span.width
        );
    }

    #[test]
    fn seam_stays_inside_span_at_extremes() {
        for r in [0.0, 1.0] {
            let split = ratio(r);
            let lead =
                (leading_extent(split, SPAN) - DIVIDER / 2.0).clamp(0.0, (SPAN - DIVIDER).max(0.0));
            assert!(lead >= 0.0 && lead + DIVIDER <= SPAN, "ratio {r}");
        }
    }

    #[test]
    fn seam_handles_span_smaller_than_divider() {
        let split = ratio(0.5);
        let span = 4.0;
        let lead =
            (leading_extent(split, span) - DIVIDER / 2.0).clamp(0.0, (span - DIVIDER).max(0.0));
        assert!(lead.is_finite() && lead >= 0.0);
    }

    #[test]
    fn seam_stays_inside_span_when_a_lock_exceeds_it() {
        let row = layout_of(Axis::Vertical, [wide(900.0), Locks::default()]);
        let span = 400.0;
        let seam = seam_center(&row, span);
        assert!(seam <= span, "seam {seam} escaped a {span} span");
    }
}
