//! Layout as plain data.
//!
//! This tree is the single source of truth for how panes are arranged, and it
//! is what gets serialised. The rendered widget is built from it every frame,
//! never the reverse, so there is only ever one representation to keep correct.
//!
//! Splits size proportionally through `Split`; pixel locks live on the panes as
//! `Locks`. That division matters. A split constrains only its own axis, but
//! which axis a pane needs held depends on its shape. A queue is a vertical
//! list that should keep its width, a timeline a horizontal strip that should
//! keep its height. Locks on the pane express either, wherever the pane sits.
//! A locked dimension holds against window resize; the unlocked one stays
//! proportional, and `MIN_SHARE` keeps a proportion from collapsing a pane out
//! of view.
//!
//! `MIN_PANE` is the floor every lock clamps to. It has to clear the tallest
//! thing any pane must still draw in its smallest form, which is the transport
//! controls at 40px, one icon at `ICON_MIN` plus padding, and it sits a little
//! above that so a pane at the floor is not exactly one icon tall.
//!
//! Its width matters to the same pane too: the transport shrinks its buttons
//! rather than dropping them, and `ICON_FLOOR` in [`crate::pane::controls`] sits
//! below what this width affords, so all three survive at the floor.
//!
//! It cannot go much higher without stopping strips from being strips. A search
//! bar draws in about 34px and a timeline in less, so the original 80px left
//! both locked to more than twice the height they use. Panes whose content wants
//! more room ask for it; the floor only says what cannot be taken away.
//!
//! `cycle_lock` advances a pane through unlocked, width, height, both. Width
//! leads because the common case is a vertical list that should keep it.
//! Holding both suits a transport strip needing a fixed height and a floor on
//! width; locks yield under pressure, so a rigid pane cannot wedge the layout.
//!
//! Releasing an axis hands the pane back to a split's ratio, and that ratio
//! still describes wherever the boundary sat before the lock, so `record_ratio`
//! rewrites it to the share the pane was actually drawn at, keeping the release
//! invisible. Two rules make that correct. A share of 1.0 means the pane fills
//! its container along that axis, so the boundary belongs to an ancestor that
//! already describes it and nothing should be written. And `governing_split`
//! returns the split *nearest* the pane rather than the topmost match, because
//! an outer split divides whole groups: writing one pane's share into it would
//! move neighbours the user never touched. The search still climbs past splits
//! on the other axis, since a pane's width and height are usually governed by
//! two different ancestors.

use serde::{Deserialize, Serialize};

use crate::pane::PaneKind;

pub const MIN_PANE: f32 = 50.0;

const MIN_SHARE: f32 = 0.05;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaneMetrics {
    pub pane: iced::Size,
    pub span: iced::Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Locks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
}

impl Locks {
    pub fn along(self, axis: Axis) -> Option<f32> {
        match axis {
            Axis::Vertical => self.width,
            Axis::Horizontal => self.height,
        }
    }

    pub fn set_along(&mut self, axis: Axis, pixels: Option<f32>) {
        match axis {
            Axis::Vertical => self.width = pixels,
            Axis::Horizontal => self.height = pixels,
        }
    }

    pub fn any(self) -> bool {
        self.width.is_some() || self.height.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Split {
    pub ratio: f32,
}

impl Default for Split {
    fn default() -> Self {
        Self { ratio: 0.5 }
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

    fn governing_split(&mut self, target: PaneId, axis: Axis) -> Option<(&mut Split, Side)> {
        let Self::Split {
            axis: inner,
            split,
            a,
            b,
        } = self
        else {
            return None;
        };

        let side = if a.contains(target) {
            Side::A
        } else if b.contains(target) {
            Side::B
        } else {
            return None;
        };

        let nearer = match side {
            Side::A => a.governing_split(target, axis),
            Side::B => b.governing_split(target, axis),
        };
        if nearer.is_some() {
            return nearer;
        }

        (*inner == axis).then_some((split, side))
    }

    fn contains(&self, target: PaneId) -> bool {
        match self {
            Self::Leaf { id } => *id == target,
            Self::Split { a, b, .. } => a.contains(target) || b.contains(target),
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
    #[serde(default, skip_serializing_if = "is_unlocked")]
    pub locks: Locks,
}

fn is_unlocked(locks: &Locks) -> bool {
    !locks.any()
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
                locks: Locks::default(),
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

    pub fn split(&mut self, target: PaneId, axis: Axis, kind: PaneKind) -> Option<PaneId> {
        let new_id = self.next_id();
        if !self.root.split_leaf(target, axis, new_id) {
            return None;
        }
        self.panes.push(PaneEntry {
            id: new_id,
            kind,
            locks: Locks::default(),
        });
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

    pub fn locks(&self, id: PaneId) -> Locks {
        self.entry(id).map(|entry| entry.locks).unwrap_or_default()
    }

    pub fn is_locked(&self, id: PaneId, axis: Axis) -> bool {
        self.locks(id).along(axis).is_some()
    }

    #[cfg(test)]
    pub fn lock(&mut self, id: PaneId, axis: Axis, size: iced::Size) {
        let extent = match axis {
            Axis::Vertical => size.width,
            Axis::Horizontal => size.height,
        };
        if let Some(entry) = self.entry_mut(id) {
            entry.locks.set_along(axis, Some(extent.max(MIN_PANE)));
        }
    }

    #[cfg(test)]
    pub fn unlock(&mut self, id: PaneId, axis: Axis) {
        if let Some(entry) = self.entry_mut(id) {
            entry.locks.set_along(axis, None);
        }
    }

    pub fn cycle_lock(&mut self, id: PaneId, size: PaneMetrics) {
        let width = || Some(size.pane.width.max(MIN_PANE));
        let height = || Some(size.pane.height.max(MIN_PANE));

        let locks = self.locks(id);
        let next = match (locks.width, locks.height) {
            (None, None) => Locks {
                width: width(),
                height: None,
            },
            (Some(_), None) => Locks {
                width: None,
                height: height(),
            },
            (None, Some(held)) => Locks {
                width: width(),
                height: Some(held),
            },
            (Some(_), Some(_)) => Locks::default(),
        };

        for axis in [Axis::Vertical, Axis::Horizontal] {
            if locks.along(axis).is_some() && next.along(axis).is_none() {
                self.record_ratio(id, axis, size);
            }
        }

        if let Some(entry) = self.entry_mut(id) {
            entry.locks = next;
        }
    }

    fn record_ratio(&mut self, id: PaneId, axis: Axis, metrics: PaneMetrics) {
        let (extent, span) = match axis {
            Axis::Vertical => (metrics.pane.width, metrics.span.width),
            Axis::Horizontal => (metrics.pane.height, metrics.span.height),
        };
        if span <= 0.0 || extent <= 0.0 {
            return;
        }

        let share = extent / span;
        if share >= 1.0 {
            return;
        }

        let Some((split, side)) = self.root.governing_split(id, axis) else {
            return;
        };

        let share = share.clamp(MIN_SHARE, 1.0 - MIN_SHARE);
        split.ratio = match side {
            Side::A => share,
            Side::B => 1.0 - share,
        };
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
        let Some(axis) = self.split_axis(path) else {
            return;
        };

        let limit = (span - MIN_PANE).max(MIN_PANE);
        let mut pinned = false;

        for (side, direction) in [(Side::A, 1.0), (Side::B, -1.0)] {
            let Some(id) = self.pinned_pane(path, side, axis) else {
                continue;
            };
            let Some(entry) = self.entry_mut(id) else {
                continue;
            };
            let current = entry.locks.along(axis).unwrap_or_default();
            let resized = (current + delta * direction).clamp(MIN_PANE, limit);
            entry.locks.set_along(axis, Some(resized));
            pinned = true;
        }

        if !pinned && let Some(split) = self.split_at_mut(path) {
            let bounded = span.max(1.0);
            split.ratio = (split.ratio + delta / bounded).clamp(MIN_SHARE, 1.0 - MIN_SHARE);
        }
    }

    fn pinned_pane(&self, path: &SplitPath, side: Side, axis: Axis) -> Option<PaneId> {
        let Node::Split { a, b, .. } = self.node_at(path)? else {
            return None;
        };
        let child = match side {
            Side::A => a,
            Side::B => b,
        };
        let Node::Leaf { id } = **child else {
            return None;
        };
        self.is_locked(id, axis).then_some(id)
    }

    fn node_at(&self, path: &SplitPath) -> Option<&Node> {
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
        Some(node)
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
                    locks: Locks::default(),
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
            split: Split { ratio: 0.62 },
            a: Box::new(Node::leaf(PaneId(0))),
            b: Box::new(Node::leaf(PaneId(1))),
        },
        panes: vec![
            PaneEntry {
                id: PaneId(0),
                kind: PaneKind::Library,
                locks: Locks::default(),
            },
            PaneEntry {
                id: PaneId(1),
                kind: PaneKind::Queue,
                locks: Locks {
                    width: Some(320.0),
                    height: None,
                },
            },
        ],
    };

    vec![browsing, Layout::single("Library", PaneKind::Library)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(width: f32, height: f32) -> iced::Size {
        iced::Size::new(width, height)
    }

    fn alone(pane: iced::Size) -> PaneMetrics {
        PaneMetrics { pane, span: pane }
    }

    fn square(extent: f32) -> iced::Size {
        size(extent, extent)
    }

    fn split_of(layout: &Layout) -> Split {
        match &layout.root {
            Node::Split { split, .. } => *split,
            Node::Leaf { .. } => panic!("expected a split at the root"),
        }
    }

    fn two_pane() -> Layout {
        Layout {
            name: "t".into(),
            root: Node::Split {
                axis: Axis::Vertical,
                split: Split::default(),
                a: Box::new(Node::leaf(PaneId(0))),
                b: Box::new(Node::leaf(PaneId(1))),
            },
            panes: vec![
                PaneEntry {
                    id: PaneId(0),
                    kind: PaneKind::Empty,
                    locks: Locks::default(),
                },
                PaneEntry {
                    id: PaneId(1),
                    kind: PaneKind::Empty,
                    locks: Locks::default(),
                },
            ],
        }
    }

    #[test]
    fn locking_width_leaves_height_free() {
        let mut l = two_pane();
        l.lock(PaneId(1), Axis::Vertical, size(320.0, 500.0));
        let locks = l.locks(PaneId(1));
        assert_eq!(locks.width, Some(320.0));
        assert_eq!(locks.height, None, "locking width pinned the height too");
    }

    #[test]
    fn locking_height_leaves_width_free() {
        let mut l = two_pane();
        l.lock(PaneId(0), Axis::Horizontal, size(900.0, 120.0));
        let locks = l.locks(PaneId(0));
        assert_eq!(locks.height, Some(120.0));
        assert_eq!(locks.width, None, "locking height pinned the width too");
    }

    #[test]
    fn each_pane_locks_independently() {
        let mut l = two_pane();
        l.lock(PaneId(0), Axis::Vertical, square(300.0));
        l.lock(PaneId(1), Axis::Vertical, square(200.0));
        assert_eq!(l.locks(PaneId(0)).width, Some(300.0));
        assert_eq!(
            l.locks(PaneId(1)).width,
            Some(200.0),
            "second lock clobbered the first"
        );
    }

    #[test]
    fn unlocking_one_axis_leaves_the_other() {
        let mut l = two_pane();
        l.lock(PaneId(0), Axis::Vertical, size(300.0, 200.0));
        l.lock(PaneId(0), Axis::Horizontal, size(300.0, 200.0));
        l.unlock(PaneId(0), Axis::Vertical);
        assert_eq!(l.locks(PaneId(0)).width, None);
        assert_eq!(l.locks(PaneId(0)).height, Some(200.0));
    }

    #[test]
    fn releasing_a_lock_keeps_the_pane_where_it_was() {
        let mut l = two_pane();
        let metrics = PaneMetrics {
            pane: size(300.0, 400.0),
            span: size(1000.0, 400.0),
        };
        l.cycle_lock(PaneId(0), metrics); // -> width
        l.cycle_lock(PaneId(0), metrics); // -> height, width released

        assert!(
            (split_of(&l).ratio - 0.3).abs() < 0.01,
            "ratio {} does not match the 0.3 share the pane held",
            split_of(&l).ratio
        );
    }

    #[test]
    fn releasing_a_lock_on_side_b_records_the_complement() {
        let mut l = two_pane();
        let metrics = PaneMetrics {
            pane: size(250.0, 400.0),
            span: size(1000.0, 400.0),
        };
        l.cycle_lock(PaneId(1), metrics);
        l.cycle_lock(PaneId(1), metrics);

        assert!(
            (split_of(&l).ratio - 0.75).abs() < 0.01,
            "ratio {} should leave side B a 0.25 share",
            split_of(&l).ratio
        );
    }

    #[test]
    fn releasing_an_axis_owned_by_an_ancestor_still_records() {
        let mut l = three_pane();
        let metrics = PaneMetrics {
            pane: size(400.0, 150.0),
            span: size(1000.0, 600.0),
        };

        l.cycle_lock(PaneId(1), metrics);
        l.cycle_lock(PaneId(1), metrics);

        let Node::Split { split, .. } = &l.root else {
            panic!("expected a split at the root");
        };
        assert!(
            (split.ratio - 0.6).abs() < 0.01,
            "root ratio {} does not leave pane 1 the 0.4 share it held",
            split.ratio
        );
    }

    #[test]
    fn no_transition_in_the_cycle_moves_a_nested_pane() {
        let mut l = three_pane();
        let metrics = PaneMetrics {
            pane: size(400.0, 150.0),
            span: size(1000.0, 600.0),
        };

        for step in 0..8 {
            l.cycle_lock(PaneId(1), metrics);

            let locks = l.locks(PaneId(1));
            if locks.width.is_none() {
                let Node::Split { split, .. } = &l.root else {
                    panic!()
                };
                assert!(
                    (split.ratio - 0.6).abs() < 0.01,
                    "step {step}: width ratio drifted to {}",
                    split.ratio
                );
            }
        }
    }

    #[test]
    fn a_recorded_ratio_reproduces_the_panes_width() {
        let mut l = two_pane();
        let span = 1000.0;
        let pane = 320.0;

        l.cycle_lock(
            PaneId(0),
            PaneMetrics {
                pane: size(pane, 400.0),
                span: size(span, 400.0),
            },
        );
        l.cycle_lock(
            PaneId(0),
            PaneMetrics {
                pane: size(pane, 400.0),
                span: size(span, 400.0),
            },
        );

        let restored = split_of(&l).ratio * span;
        assert!(
            (restored - pane).abs() < 1.0,
            "released pane would render at {restored}, not the {pane} it held"
        );
    }

    #[test]
    fn cycling_into_free_records_both_axes() {
        let mut l = three_pane();
        let metrics = PaneMetrics {
            pane: size(400.0, 150.0),
            span: size(1000.0, 600.0),
        };

        l.cycle_lock(PaneId(1), metrics);
        l.cycle_lock(PaneId(1), metrics);
        l.cycle_lock(PaneId(1), metrics);
        assert!(l.locks(PaneId(1)).width.is_some() && l.locks(PaneId(1)).height.is_some());

        l.cycle_lock(PaneId(1), metrics);
        assert!(!l.locks(PaneId(1)).any(), "expected free");

        let Node::Split { split, b, .. } = &l.root else {
            panic!("expected a split at the root");
        };
        assert!(
            (split.ratio - 0.6).abs() < 0.01,
            "width ratio {} does not leave pane 1 a 0.4 share",
            split.ratio
        );

        let Node::Split { split: inner, .. } = &**b else {
            panic!("expected a nested split");
        };
        assert!(
            (inner.ratio - 0.25).abs() < 0.01,
            "height ratio {} does not leave pane 1 a 0.25 share",
            inner.ratio
        );
    }

    #[test]
    fn releasing_an_ungoverned_axis_leaves_the_other_alone() {
        let mut l = two_pane();
        let metrics = PaneMetrics {
            pane: size(300.0, 600.0),
            span: size(1000.0, 600.0),
        };

        l.cycle_lock(PaneId(0), metrics); // width
        l.cycle_lock(PaneId(0), metrics); // height, width released
        let after_width_release = split_of(&l).ratio;

        l.cycle_lock(PaneId(0), metrics); // both
        l.cycle_lock(PaneId(0), metrics); // free: releases width AND height

        let ratio = split_of(&l).ratio;
        assert!(
            (ratio - 0.3).abs() < 0.01,
            "width ratio {ratio} drifted from {after_width_release}; the height \
             release should not touch a vertical split"
        );
    }

    #[test]
    fn a_nested_pane_freed_does_not_snap_to_the_clamp() {
        let mut l = three_pane();
        let metrics = PaneMetrics {
            pane: size(500.0, 300.0),
            span: size(500.0, 600.0),
        };

        l.cycle_lock(PaneId(1), metrics); // width
        l.cycle_lock(PaneId(1), metrics); // height
        l.cycle_lock(PaneId(1), metrics); // both
        l.cycle_lock(PaneId(1), metrics); // free

        let Node::Split { split, .. } = &l.root else {
            panic!("expected a split at the root");
        };
        assert!(
            (split.ratio - 0.5).abs() < 0.01,
            "root ratio snapped to {}; pane 1 still occupies half the window",
            split.ratio
        );
    }

    #[test]
    fn freeing_a_pane_in_a_three_pane_row_leaves_its_neighbours_alone() {
        let mut l = two_pane();
        l.split(PaneId(0), Axis::Vertical, PaneKind::Empty); // 0 -> (0|2)

        let inner_before = match &l.root {
            Node::Split { a, .. } => match &**a {
                Node::Split { split, .. } => split.ratio,
                Node::Leaf { .. } => panic!("expected a nested split"),
            },
            Node::Leaf { .. } => panic!("expected a split"),
        };

        let metrics = PaneMetrics {
            pane: size(250.0, 600.0),
            span: size(1000.0, 600.0),
        };
        l.cycle_lock(PaneId(1), metrics);
        l.cycle_lock(PaneId(1), metrics);

        let inner_after = match &l.root {
            Node::Split { a, .. } => match &**a {
                Node::Split { split, .. } => split.ratio,
                Node::Leaf { .. } => panic!(),
            },
            Node::Leaf { .. } => panic!(),
        };

        assert!(
            (inner_after - inner_before).abs() < f32::EPSILON,
            "freeing pane 1 moved the 0|2 split from {inner_before} to {inner_after}"
        );
    }

    #[test]
    fn freeing_a_nested_pane_adjusts_its_own_split() {
        let mut l = two_pane();
        l.split(PaneId(0), Axis::Vertical, PaneKind::Empty); // ((0|2)|1)

        let outer_before = split_of(&l).ratio;

        let metrics = PaneMetrics {
            pane: size(300.0, 600.0),
            span: size(700.0, 600.0),
        };
        l.cycle_lock(PaneId(2), metrics);
        l.cycle_lock(PaneId(2), metrics);

        assert!(
            (split_of(&l).ratio - outer_before).abs() < f32::EPSILON,
            "freeing pane 2 moved the outer split from {outer_before} to {}",
            split_of(&l).ratio
        );
    }

    #[test]
    fn cycle_lock_walks_the_four_states() {
        let mut l = two_pane();
        let pane = size(320.0, 180.0);

        l.cycle_lock(PaneId(0), alone(pane));
        assert_eq!(l.locks(PaneId(0)).width, Some(320.0));
        assert_eq!(l.locks(PaneId(0)).height, None);

        l.cycle_lock(PaneId(0), alone(pane));
        assert_eq!(l.locks(PaneId(0)).width, None);
        assert_eq!(l.locks(PaneId(0)).height, Some(180.0));

        l.cycle_lock(PaneId(0), alone(pane));
        assert_eq!(l.locks(PaneId(0)).width, Some(320.0));
        assert_eq!(l.locks(PaneId(0)).height, Some(180.0));

        l.cycle_lock(PaneId(0), alone(pane));
        assert!(
            !l.locks(PaneId(0)).any(),
            "cycle did not return to unlocked"
        );
    }

    #[test]
    fn cycle_lock_returns_to_where_it_started() {
        let mut l = two_pane();
        let pane = size(320.0, 180.0);
        for _ in 0..4 {
            l.cycle_lock(PaneId(0), alone(pane));
        }
        assert!(!l.locks(PaneId(0)).any());
        for _ in 0..4 {
            l.cycle_lock(PaneId(0), alone(pane));
        }
        assert!(!l.locks(PaneId(0)).any(), "cycle length is not four");
    }

    #[test]
    fn cycling_to_both_keeps_the_height_already_held() {
        let mut l = two_pane();
        l.cycle_lock(PaneId(0), alone(size(320.0, 180.0)));
        l.cycle_lock(PaneId(0), alone(size(320.0, 180.0)));
        l.cycle_lock(PaneId(0), alone(size(999.0, 999.0)));
        assert_eq!(l.locks(PaneId(0)).height, Some(180.0));
    }

    #[test]
    fn dragging_sets_the_locked_width() {
        let mut l = two_pane();
        l.lock(PaneId(0), Axis::Vertical, square(240.0));
        let path = SplitPath(vec![]);

        l.drag_divider(&path, 70.0, 1000.0);
        assert_eq!(l.locks(PaneId(0)).width, Some(310.0));

        l.drag_divider(&path, -110.0, 1000.0);
        assert_eq!(l.locks(PaneId(0)).width, Some(200.0));
    }

    #[test]
    fn dragging_leaves_the_unlocked_axis_alone() {
        let mut l = two_pane();
        l.lock(PaneId(0), Axis::Vertical, size(240.0, 150.0));
        l.drag_divider(&SplitPath(vec![]), 70.0, 1000.0);
        assert_eq!(
            l.locks(PaneId(0)).height,
            None,
            "a width drag created a height lock"
        );
    }

    #[test]
    fn dragging_a_locked_side_b_moves_the_opposite_way() {
        let mut l = two_pane();
        l.lock(PaneId(1), Axis::Vertical, square(200.0));
        l.drag_divider(&SplitPath(vec![]), 60.0, 1000.0);
        assert_eq!(l.locks(PaneId(1)).width, Some(140.0));
    }

    #[test]
    fn dragging_a_locked_pane_leaves_the_ratio_alone() {
        let mut l = two_pane();
        let before = split_of(&l).ratio;
        l.lock(PaneId(0), Axis::Vertical, square(240.0));
        l.drag_divider(&SplitPath(vec![]), 75.0, 1000.0);
        let after = split_of(&l).ratio;
        assert!(
            (after - before).abs() < f32::EPSILON,
            "ratio drifted {before} -> {after} while locked"
        );
    }

    #[test]
    fn dragging_never_takes_a_locked_pane_below_min_pane() {
        let mut l = two_pane();
        l.lock(PaneId(0), Axis::Vertical, square(240.0));
        l.drag_divider(&SplitPath(vec![]), -1000.0, 1000.0);
        let width = l.locks(PaneId(0)).width.expect("still locked");
        assert!(width >= MIN_PANE, "got {width}");
    }

    #[test]
    fn unlocking_makes_the_divider_move_the_ratio_again() {
        let mut l = two_pane();
        l.lock(PaneId(0), Axis::Vertical, square(240.0));
        l.unlock(PaneId(0), Axis::Vertical);
        l.drag_divider(&SplitPath(vec![]), 100.0, 1000.0);
        assert!((split_of(&l).ratio - 0.6).abs() < 0.01);
    }

    #[test]
    fn locks_are_independent_across_nested_splits() {
        let mut l = three_pane();
        l.lock(PaneId(0), Axis::Vertical, square(240.0));
        l.lock(PaneId(1), Axis::Vertical, square(150.0));
        l.lock(PaneId(2), Axis::Horizontal, square(180.0));
        assert_eq!(l.locks(PaneId(0)).width, Some(240.0));
        assert_eq!(l.locks(PaneId(1)).width, Some(150.0));
        assert_eq!(l.locks(PaneId(2)).height, Some(180.0));
    }

    #[test]
    fn lock_never_records_less_than_min_pane() {
        let mut l = two_pane();
        l.lock(PaneId(0), Axis::Vertical, square(10.0));
        let width = l.locks(PaneId(0)).width.expect("locked");
        assert!(width >= MIN_PANE, "got {width}");
    }

    #[test]
    fn ratio_drag_moves_boundary_by_cursor_delta() {
        for span in [1200.0, 768.0, 400.0] {
            for delta in [1.0, 17.0, -23.0] {
                let mut l = two_pane();
                let path = SplitPath(vec![]);

                let before = split_of(&l).ratio * span;
                l.drag_divider(&path, delta, span);
                let after = split_of(&l).ratio * span;

                assert!(
                    (after - before - delta).abs() < 0.01,
                    "span {span}, delta {delta}: boundary moved {} not {delta}",
                    after - before
                );
            }
        }
    }

    #[test]
    fn ratio_drag_and_clamp() {
        let mut l = two_pane();
        let path = SplitPath(vec![]);
        l.drag_divider(&path, 100.0, 1000.0);
        let ratio = split_of(&l).ratio;
        assert!((ratio - 0.6).abs() < 0.01, "got {ratio}");
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
    fn a_dragged_lock_size_survives_a_save_and_reload() {
        let mut l = two_pane();
        l.lock(PaneId(0), Axis::Vertical, square(240.0));
        l.drag_divider(&SplitPath(vec![]), 85.0, 1000.0);

        let text = toml::to_string(&l).unwrap();
        let back: Layout = toml::from_str(&text).unwrap();

        assert_eq!(back.locks(PaneId(0)).width, Some(325.0));
    }

    #[test]
    fn unlocked_survives_toml_round_trip() {
        let l = two_pane();
        let s = toml::to_string(&l).unwrap();
        let back: Layout = toml::from_str(&s).unwrap();
        assert_eq!(l, back);
        assert!(!back.locks(PaneId(0)).any());
    }

    #[test]
    fn locked_survives_toml_round_trip() {
        let mut l = two_pane();
        l.lock(PaneId(0), Axis::Vertical, square(260.0));
        let s = toml::to_string(&l).unwrap();
        let back: Layout = toml::from_str(&s).unwrap();
        assert_eq!(l, back);
        assert_eq!(back.locks(PaneId(0)).width, Some(260.0));
    }
}
