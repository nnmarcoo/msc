//! Bridge between the layout data and `pane_grid`'s widget state.
//!
//! [`build`] projects the layout into a grid; [`write_back`] folds the results
//! of a drag or resize back into the layout. The layout stays authoritative —
//! nothing here is read as a source of truth, only as the outcome of a gesture.

use iced::widget::pane_grid::{self, Configuration};

use crate::layout::{Axis, Layout, Node, PaneId};

pub fn build(layout: &Layout) -> pane_grid::State<PaneId> {
    pane_grid::State::with_configuration(configuration(&layout.root))
}

fn configuration(node: &Node) -> Configuration<PaneId> {
    match node {
        Node::Leaf { id } => Configuration::Pane(*id),
        Node::Split { axis, ratio, a, b } => Configuration::Split {
            axis: match axis {
                Axis::Horizontal => pane_grid::Axis::Horizontal,
                Axis::Vertical => pane_grid::Axis::Vertical,
            },
            ratio: *ratio,
            a: Box::new(configuration(a)),
            b: Box::new(configuration(b)),
        },
    }
}

/// The tree the grid currently holds, ready to replace a layout's root. Pane
/// entries are untouched, so kinds and settings survive a rearrangement.
pub fn read_back(state: &pane_grid::State<PaneId>) -> Option<Node> {
    from_node(state, state.layout())
}

fn from_node(state: &pane_grid::State<PaneId>, node: &pane_grid::Node) -> Option<Node> {
    match node {
        pane_grid::Node::Pane(pane) => state.get(*pane).map(|id| Node::leaf(*id)),
        pane_grid::Node::Split {
            axis, ratio, a, b, ..
        } => {
            let a = from_node(state, a)?;
            let b = from_node(state, b)?;
            Some(Node::Split {
                axis: match axis {
                    pane_grid::Axis::Horizontal => Axis::Horizontal,
                    pane_grid::Axis::Vertical => Axis::Vertical,
                },
                ratio: *ratio,
                a: Box::new(a),
                b: Box::new(b),
            })
        }
    }
}
