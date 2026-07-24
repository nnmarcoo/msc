//! Layout as plain data.
//!
//! Deliberately free of iced types: this tree is the single source of truth for
//! how panes are arranged, and it is what gets serialised. The rendered widget
//! is built from it every frame, never the reverse, so there is only ever one
//! representation of the layout to keep correct.
//!
//! Sizing lives on each split, not on panes. A split divides its space between
//! two children according to a [`Split`](Split): either a proportional ratio,
//! or one side locked to a pixel size while the other takes the remainder. A
//! locked side holds against window growth and yields only when the window is
//! too small to fit everything.

use serde::{Deserialize, Serialize};

use crate::pane::PaneKind;

pub const MIN_PANE: f32 = 80.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PaneId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    A,
    B,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitPath(pub Vec<Side>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropZone {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

impl DropZone {
    pub fn from_fraction(fx: f32, fy: f32) -> Self {
        const EDGE: f32 = 1.0 / 3.0;

        let candidates = [
            (Self::Left, fx),
            (Self::Right, 1.0 - fx),
            (Self::Top, fy),
            (Self::Bottom, 1.0 - fy),
        ];

        let (zone, distance) = candidates
            .into_iter()
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .unwrap_or((Self::Center, 1.0));

        if distance > EDGE { Self::Center } else { zone }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Split {
    Ratio { ratio: f32 },
    Locked { side: Side, pixels: f32 },
}

impl Default for Split {
    fn default() -> Self {
        Self::Ratio { ratio: 0.5 }
    }
}

impl Split {
    pub fn locked_side(self) -> Option<Side> {
        match self {
            Self::Locked { side, .. } => Some(side),
            Self::Ratio { .. } => None,
        }
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
        #[serde(default)]
        split: Split,
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
                    split: Split::default(),
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

    fn replace_leaf_with_split(
        &mut self,
        target: PaneId,
        axis: Axis,
        moved: PaneId,
        moved_first: bool,
    ) -> bool {
        match self {
            Self::Leaf { id } if *id == target => {
                let (a, b) = if moved_first {
                    (Self::leaf(moved), Self::leaf(target))
                } else {
                    (Self::leaf(target), Self::leaf(moved))
                };
                *self = Self::Split {
                    axis,
                    split: Split::default(),
                    a: Box::new(a),
                    b: Box::new(b),
                };
                true
            }
            Self::Leaf { .. } => false,
            Self::Split { a, b, .. } => {
                a.replace_leaf_with_split(target, axis, moved, moved_first)
                    || b.replace_leaf_with_split(target, axis, moved, moved_first)
            }
        }
    }

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

    fn parent_of(&self, target: PaneId) -> Option<(Split, Side)> {
        match self {
            Self::Leaf { .. } => None,
            Self::Split { split, a, b, .. } => {
                if matches!(**a, Self::Leaf { id } if id == target) {
                    return Some((*split, Side::A));
                }
                if matches!(**b, Self::Leaf { id } if id == target) {
                    return Some((*split, Side::B));
                }
                a.parent_of(target).or_else(|| b.parent_of(target))
            }
        }
    }

    fn parent_axis_of(&self, target: PaneId) -> Option<Axis> {
        match self {
            Self::Leaf { .. } => None,
            Self::Split { axis, a, b, .. } => {
                if matches!(**a, Self::Leaf { id } if id == target)
                    || matches!(**b, Self::Leaf { id } if id == target)
                {
                    return Some(*axis);
                }
                a.parent_axis_of(target)
                    .or_else(|| b.parent_axis_of(target))
            }
        }
    }

    fn with_parent(&mut self, target: PaneId, f: &mut dyn FnMut(&mut Split, Side)) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split { split, a, b, .. } => {
                let side = if matches!(**a, Self::Leaf { id } if id == target) {
                    Some(Side::A)
                } else if matches!(**b, Self::Leaf { id } if id == target) {
                    Some(Side::B)
                } else {
                    None
                };
                if let Some(side) = side {
                    f(split, side);
                    return true;
                }
                a.with_parent(target, f) || b.with_parent(target, f)
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
}

impl Layout {
    pub fn single(name: impl Into<String>, kind: PaneKind) -> Self {
        let id = PaneId(0);
        Self {
            name: name.into(),
            root: Node::leaf(id),
            panes: vec![PaneEntry { id, kind }],
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

    pub fn split(&mut self, target: PaneId, axis: Axis, kind: PaneKind) -> Option<PaneId> {
        let new_id = self.next_id();
        if !self.root.split_leaf(target, axis, new_id) {
            return None;
        }
        self.panes.push(PaneEntry { id: new_id, kind });
        Some(new_id)
    }

    pub fn close(&mut self, target: PaneId) -> bool {
        if self.panes.len() <= 1 || !self.root.remove_leaf(target) {
            return false;
        }
        self.panes.retain(|entry| entry.id != target);
        true
    }

    pub fn move_pane(&mut self, source: PaneId, target: PaneId, zone: DropZone) -> bool {
        if source == target || self.entry(source).is_none() || self.entry(target).is_none() {
            return false;
        }

        if zone == DropZone::Center {
            return self.swap_panes(source, target);
        }

        let (axis, source_first) = match zone {
            DropZone::Left => (Axis::Vertical, true),
            DropZone::Right => (Axis::Vertical, false),
            DropZone::Top => (Axis::Horizontal, true),
            DropZone::Bottom => (Axis::Horizontal, false),
            DropZone::Center => unreachable!(),
        };

        if !self.root.remove_leaf(source) {
            return false;
        }
        self.root
            .replace_leaf_with_split(target, axis, source, source_first);
        true
    }

    pub fn move_pane_to_root_edge(&mut self, source: PaneId, edge: DropZone) -> bool {
        let (axis, source_first) = match edge {
            DropZone::Left => (Axis::Vertical, true),
            DropZone::Right => (Axis::Vertical, false),
            DropZone::Top => (Axis::Horizontal, true),
            DropZone::Bottom => (Axis::Horizontal, false),
            DropZone::Center => return false,
        };

        if self.panes.len() < 2 || self.entry(source).is_none() {
            return false;
        }

        if !self.root.remove_leaf(source) {
            return false;
        }

        let rest = std::mem::replace(&mut self.root, Node::leaf(source));
        let moved = Box::new(Node::leaf(source));
        let rest = Box::new(rest);
        let (a, b) = if source_first {
            (moved, rest)
        } else {
            (rest, moved)
        };
        self.root = Node::Split {
            axis,
            split: Split::default(),
            a,
            b,
        };
        true
    }

    fn swap_panes(&mut self, a: PaneId, b: PaneId) -> bool {
        let kind_a = self.kind(a);
        let kind_b = self.kind(b);
        if let (Some(ka), Some(kb)) = (kind_a, kind_b) {
            self.set_kind(a, kb);
            self.set_kind(b, ka);
            return true;
        }
        false
    }

    pub fn is_locked(&self, id: PaneId) -> bool {
        self.root
            .parent_of(id)
            .is_some_and(|(split, side)| split.locked_side() == Some(side))
    }

    /// The axis of the split that `id` is a child of, if any. A vertical split
    /// arranges its sides left/right (so locking pins a width); a horizontal
    /// split arranges them top/bottom (locking pins a height).
    pub fn parent_axis(&self, id: PaneId) -> Option<Axis> {
        self.root.parent_axis_of(id)
    }

    pub fn lock(&mut self, id: PaneId, pixels: f32) {
        self.root.with_parent(id, &mut |split, side| {
            *split = Split::Locked {
                side,
                pixels: pixels.max(MIN_PANE),
            };
        });
    }

    pub fn unlock(&mut self, id: PaneId) {
        self.root.with_parent(id, &mut |split, _side| {
            *split = Split::Ratio { ratio: 0.5 };
        });
    }

    pub fn split_axis(&self, path: &SplitPath) -> Option<Axis> {
        let mut node = &self.root;
        for step in &path.0 {
            let Node::Split { a, b, .. } = node else {
                return None;
            };
            node = match step {
                Side::A => a,
                Side::B => b,
            };
        }
        match node {
            Node::Split { axis, .. } => Some(*axis),
            Node::Leaf { .. } => None,
        }
    }

    pub fn drag_divider(&mut self, path: &SplitPath, delta: f32, span: f32) {
        let Some(split) = self.split_at_mut(path) else {
            return;
        };
        match split {
            Split::Locked { side, pixels } => {
                let signed = if *side == Side::A { delta } else { -delta };
                *pixels = (*pixels + signed).clamp(MIN_PANE, span - MIN_PANE);
            }
            Split::Ratio { ratio } => {
                let bounded = span.max(1.0);
                *ratio = (*ratio + delta / bounded).clamp(0.05, 0.95);
            }
        }
    }

    fn split_at_mut(&mut self, path: &SplitPath) -> Option<&mut Split> {
        let mut node = &mut self.root;
        for step in &path.0 {
            let Node::Split { a, b, .. } = node else {
                return None;
            };
            node = match step {
                Side::A => a,
                Side::B => b,
            };
        }
        match node {
            Node::Split { split, .. } => Some(split),
            Node::Leaf { .. } => None,
        }
    }

    pub fn reconcile(&mut self) {
        let live = self.root.pane_ids();
        self.panes.retain(|entry| live.contains(&entry.id));

        for id in live {
            if self.entry(id).is_none() {
                self.panes.push(PaneEntry {
                    id,
                    kind: PaneKind::Empty,
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
            split: Split::Ratio { ratio: 0.62 },
            a: Box::new(Node::leaf(PaneId(0))),
            b: Box::new(Node::leaf(PaneId(1))),
        },
        panes: vec![
            PaneEntry {
                id: PaneId(0),
                kind: PaneKind::Library,
            },
            PaneEntry {
                id: PaneId(1),
                kind: PaneKind::Queue,
            },
        ],
    };

    vec![browsing, Layout::single("Library", PaneKind::Library)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_pane() -> Layout {
        Layout {
            name: "t".into(),
            root: Node::Split {
                axis: Axis::Vertical,
                split: Split::Ratio { ratio: 0.5 },
                a: Box::new(Node::leaf(PaneId(0))),
                b: Box::new(Node::leaf(PaneId(1))),
            },
            panes: vec![
                PaneEntry {
                    id: PaneId(0),
                    kind: PaneKind::Empty,
                },
                PaneEntry {
                    id: PaneId(1),
                    kind: PaneKind::Empty,
                },
            ],
        }
    }

    #[test]
    fn lock_side_a_then_drag_sets_pixels() {
        let mut l = two_pane();
        l.lock(PaneId(0), 240.0);
        assert!(l.is_locked(PaneId(0)));

        let path = SplitPath(vec![]);
        l.drag_divider(&path, 50.0, 1000.0);
        match l.root {
            Node::Split {
                split:
                    Split::Locked {
                        side: Side::A,
                        pixels,
                    },
                ..
            } => {
                assert!((pixels - 290.0).abs() < 0.01, "got {pixels}");
            }
            _ => panic!("expected locked A"),
        }
    }

    #[test]
    fn lock_side_b_drag_moves_correct_direction() {
        let mut l = two_pane();
        l.lock(PaneId(1), 200.0);
        let path = SplitPath(vec![]);
        l.drag_divider(&path, 60.0, 1000.0);
        match l.root {
            Node::Split {
                split:
                    Split::Locked {
                        side: Side::B,
                        pixels,
                    },
                ..
            } => {
                assert!((pixels - 140.0).abs() < 0.01, "got {pixels}");
            }
            _ => panic!("expected locked B"),
        }
    }

    #[test]
    fn ratio_drag_and_clamp() {
        let mut l = two_pane();
        let path = SplitPath(vec![]);
        l.drag_divider(&path, 100.0, 1000.0);
        match l.root {
            Node::Split {
                split: Split::Ratio { ratio },
                ..
            } => {
                assert!((ratio - 0.6).abs() < 0.01, "got {ratio}");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn min_pane_clamp_under_pressure() {
        let mut l = two_pane();
        l.lock(PaneId(0), 240.0);
        let path = SplitPath(vec![]);
        l.drag_divider(&path, -1000.0, 500.0);
        match l.root {
            Node::Split {
                split: Split::Locked { pixels, .. },
                ..
            } => {
                assert!(pixels >= MIN_PANE, "got {pixels}");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn drop_zone_from_fraction() {
        assert_eq!(DropZone::from_fraction(0.5, 0.5), DropZone::Center);
        assert_eq!(DropZone::from_fraction(0.05, 0.5), DropZone::Left);
        assert_eq!(DropZone::from_fraction(0.95, 0.5), DropZone::Right);
        assert_eq!(DropZone::from_fraction(0.5, 0.05), DropZone::Top);
        assert_eq!(DropZone::from_fraction(0.5, 0.95), DropZone::Bottom);
    }

    #[test]
    fn move_pane_edge_creates_split() {
        let mut l = two_pane();
        assert!(l.move_pane(PaneId(1), PaneId(0), DropZone::Left));
        assert_eq!(l.panes.len(), 2);
        assert!(l.entry(PaneId(0)).is_some() && l.entry(PaneId(1)).is_some());
        match &l.root {
            Node::Split {
                axis: Axis::Vertical,
                a,
                b,
                ..
            } => {
                assert!(matches!(**a, Node::Leaf { id } if id == PaneId(1)));
                assert!(matches!(**b, Node::Leaf { id } if id == PaneId(0)));
            }
            _ => panic!("expected vertical split, got {:?}", l.root),
        }
    }

    #[test]
    fn move_pane_center_swaps_kinds() {
        let mut l = two_pane();
        l.set_kind(PaneId(0), PaneKind::Library);
        l.set_kind(PaneId(1), PaneKind::Queue);
        assert!(l.move_pane(PaneId(0), PaneId(1), DropZone::Center));
        assert_eq!(l.kind(PaneId(0)), Some(PaneKind::Queue));
        assert_eq!(l.kind(PaneId(1)), Some(PaneKind::Library));
    }

    #[test]
    fn move_pane_onto_self_is_noop() {
        let mut l = two_pane();
        assert!(!l.move_pane(PaneId(0), PaneId(0), DropZone::Left));
    }

    #[test]
    fn move_pane_stays_valid_tree() {
        let mut l = two_pane();
        l.move_pane(PaneId(1), PaneId(0), DropZone::Bottom);
        let mut ids = l.root.pane_ids();
        ids.sort();
        let mut entries: Vec<_> = l.panes.iter().map(|e| e.id).collect();
        entries.sort();
        assert_eq!(ids, entries);
    }

    fn three_pane() -> Layout {
        let mut l = two_pane();
        l.split(PaneId(1), Axis::Horizontal, PaneKind::Empty);
        l
    }

    #[test]
    fn root_edge_bottom_wraps_everything() {
        let mut l = three_pane();
        let ids_before = {
            let mut v = l.root.pane_ids();
            v.sort();
            v
        };
        assert!(l.move_pane_to_root_edge(PaneId(0), DropZone::Bottom));
        match &l.root {
            Node::Split {
                axis: Axis::Horizontal,
                a,
                b,
                ..
            } => {
                assert!(matches!(**b, Node::Leaf { id } if id == PaneId(0)));
                let mut rest = a.pane_ids();
                rest.sort();
                assert_eq!(rest, vec![PaneId(1), PaneId(2)]);
            }
            other => panic!("expected horizontal root split, got {other:?}"),
        }
        let mut ids_after = l.root.pane_ids();
        ids_after.sort();
        assert_eq!(ids_before, ids_after);
    }

    #[test]
    fn root_edge_left_puts_pane_first() {
        let mut l = three_pane();
        assert!(l.move_pane_to_root_edge(PaneId(2), DropZone::Left));
        match &l.root {
            Node::Split {
                axis: Axis::Vertical,
                a,
                ..
            } => {
                assert!(matches!(**a, Node::Leaf { id } if id == PaneId(2)));
            }
            other => panic!("expected vertical root split, got {other:?}"),
        }
    }

    #[test]
    fn root_edge_center_rejected() {
        let mut l = three_pane();
        assert!(!l.move_pane_to_root_edge(PaneId(0), DropZone::Center));
    }

    #[test]
    fn root_edge_single_pane_noop() {
        let mut l = Layout::single("s", PaneKind::Library);
        assert!(!l.move_pane_to_root_edge(PaneId(0), DropZone::Top));
    }

    #[test]
    fn locked_survives_toml_round_trip() {
        let mut l = two_pane();
        l.lock(PaneId(0), 260.0);
        let s = toml::to_string(&l).unwrap();
        let back: Layout = toml::from_str(&s).unwrap();
        assert_eq!(l, back);
        assert!(back.is_locked(PaneId(0)));
    }
}
