//! Layout as plain data.
//!
//! Deliberately free of iced types: this tree is the single source of truth for
//! how panes are arranged, and it is what gets serialised. The widget state is
//! rebuilt from it whenever it changes, never the other way around, so there is
//! only ever one representation of the layout to keep correct.

use serde::{Deserialize, Serialize};

use crate::pane::PaneKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PaneId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Sizing {
    Fill,
    Fixed { pixels: f32 },
}

impl Default for Sizing {
    fn default() -> Self {
        Self::Fill
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Node {
    Leaf {
        id: PaneId,
    },
    Split {
        axis: Axis,
        ratio: f32,
        a: Box<Node>,
        b: Box<Node>,
    },
}

impl Node {
    pub fn leaf(id: PaneId) -> Self {
        Self::Leaf { id }
    }

    pub fn pane_ids(&self) -> Vec<PaneId> {
        let mut ids = Vec::new();
        self.collect_ids(&mut ids);
        ids
    }

    fn collect_ids(&self, out: &mut Vec<PaneId>) {
        match self {
            Self::Leaf { id } => out.push(*id),
            Self::Split { a, b, .. } => {
                a.collect_ids(out);
                b.collect_ids(out);
            }
        }
    }

    fn split_leaf(&mut self, target: PaneId, axis: Axis, new_id: PaneId) -> bool {
        match self {
            Self::Leaf { id } if *id == target => {
                *self = Self::Split {
                    axis,
                    ratio: 0.5,
                    a: Box::new(Self::leaf(target)),
                    b: Box::new(Self::leaf(new_id)),
                };
                true
            }
            Self::Leaf { .. } => false,
            Self::Split { a, b, .. } => {
                a.split_leaf(target, axis, new_id) || b.split_leaf(target, axis, new_id)
            }
        }
    }

    /// Collapses the split that contained `target`, promoting its sibling.
    /// Returns `false` when `target` is the last remaining pane.
    fn remove_leaf(&mut self, target: PaneId) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split { a, b, .. } => {
                if matches!(**a, Self::Leaf { id } if id == target) {
                    *self = (**b).clone();
                    return true;
                }
                if matches!(**b, Self::Leaf { id } if id == target) {
                    *self = (**a).clone();
                    return true;
                }
                a.remove_leaf(target) || b.remove_leaf(target)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub name: String,
    pub root: Node,
    pub panes: Vec<PaneEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaneEntry {
    pub id: PaneId,
    pub kind: PaneKind,
    #[serde(default)]
    pub sizing: Sizing,
}

impl Layout {
    pub fn single(name: impl Into<String>, kind: PaneKind) -> Self {
        let id = PaneId(0);
        Self {
            name: name.into(),
            root: Node::leaf(id),
            panes: vec![PaneEntry {
                id,
                kind,
                sizing: Sizing::Fill,
            }],
        }
    }

    pub fn kind(&self, id: PaneId) -> Option<PaneKind> {
        self.entry(id).map(|entry| entry.kind)
    }

    pub fn entry(&self, id: PaneId) -> Option<&PaneEntry> {
        self.panes.iter().find(|entry| entry.id == id)
    }

    pub fn entry_mut(&mut self, id: PaneId) -> Option<&mut PaneEntry> {
        self.panes.iter_mut().find(|entry| entry.id == id)
    }

    pub fn set_kind(&mut self, id: PaneId, kind: PaneKind) {
        if let Some(entry) = self.entry_mut(id) {
            entry.kind = kind;
        }
    }

    pub fn len(&self) -> usize {
        self.panes.len()
    }

    pub fn split(&mut self, target: PaneId, axis: Axis, kind: PaneKind) -> Option<PaneId> {
        let new_id = self.next_id();
        if !self.root.split_leaf(target, axis, new_id) {
            return None;
        }
        self.panes.push(PaneEntry {
            id: new_id,
            kind,
            sizing: Sizing::Fill,
        });
        Some(new_id)
    }

    /// Refuses to remove the final pane, so a layout is never empty.
    pub fn close(&mut self, target: PaneId) -> bool {
        if self.panes.len() <= 1 || !self.root.remove_leaf(target) {
            return false;
        }
        self.panes.retain(|entry| entry.id != target);
        true
    }

    /// Drops entries the tree no longer references, and adopts any leaf that
    /// somehow lacks one. Guards against a hand-edited config file leaving the
    /// two halves inconsistent.
    pub fn reconcile(&mut self) {
        let live = self.root.pane_ids();
        self.panes.retain(|entry| live.contains(&entry.id));

        for id in live {
            if self.entry(id).is_none() {
                self.panes.push(PaneEntry {
                    id,
                    kind: PaneKind::Empty,
                    sizing: Sizing::Fill,
                });
            }
        }
    }

    fn next_id(&self) -> PaneId {
        PaneId(
            self.panes
                .iter()
                .map(|entry| entry.id.0)
                .max()
                .map_or(0, |max| max + 1),
        )
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::single("Default", PaneKind::Library)
    }
}

pub fn default_presets() -> Vec<Layout> {
    let browsing = Layout {
        name: "Browsing".into(),
        root: Node::Split {
            axis: Axis::Vertical,
            ratio: 0.62,
            a: Box::new(Node::leaf(PaneId(0))),
            b: Box::new(Node::leaf(PaneId(1))),
        },
        panes: vec![
            PaneEntry {
                id: PaneId(0),
                kind: PaneKind::Library,
                sizing: Sizing::Fill,
            },
            PaneEntry {
                id: PaneId(1),
                kind: PaneKind::Queue,
                sizing: Sizing::Fill,
            },
        ],
    };

    vec![browsing, Layout::single("Library", PaneKind::Library)]
}
