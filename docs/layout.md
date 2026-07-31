# Layout system plan

A pane system where the user builds their own layout: split, resize, drag, and
configure each pane's contents. Written against the rewritten `verse-gui`.

> **Status: Option A shipped.** Layout data model, enum-based panes, per-pane
> messages, edit mode (split/close/change-kind/drag/resize), named presets, and
> persistence are all in. The custom-widget path (Option B) remains available
> without touching the data model. Remaining: fixed-shell regions beyond the
> transport bar, per-pane settings popovers, more pane kinds.

## What went wrong last time

The old system worked, but was hard to follow. Four concrete causes, each of
which the new design removes:

**1. Pane content was a trait object.** `Box<dyn PaneView>` meant no single place
listed what a pane can hold, and the compiler could not check that every pane
type was handled.

**2. Which forced downcasting.** To reach a specific pane's state, `app.rs` did
`pane.content.as_any_mut().downcast_mut::<CollectionsPane>()` — five times. Each
site silently does nothing if the pane is not that type, so a typo or a
refactor fails silently rather than at compile time.

**3. Which forced broadcast updates.** Because the app could not address one
pane, messages were applied by looping over *every* pane and downcasting. A
message meant for one Collections pane hit all of them.

**4. Layout was stored three ways.** `pane_grid::State` (live), `Vec<Configuration>`
(presets), and `Vec<LayoutNode>` (on disk), with conversions in both directions
(`pane_config_to_node` / `node_to_pane_config`). Three representations of one
fact, kept in sync by hand.

## The hard constraint

`iced::widget::pane_grid` cannot express "this pane stays a fixed size".

- Splits store a **ratio** (`0.0..=1.0`), not pixels — see `Node::Split { ratio }`.
  Resizing the window rescales every pane proportionally.
- `min_size` is a **single global value** for the whole grid (default `50.0`),
  not per-pane.

So a fixed-width sidebar and a filling library cannot both be `pane_grid` panes.
This is the central design decision, and the options are laid out below.

---

# Design

## Layer 1 — Layout data, separate from widgets

Following Halloy's split (`data/src/dashboard.rs` owns the pane tree, distinct
from the UI code), layout becomes plain data with no iced types in it:

```rust
pub struct Layout {
    root: Node,
    focus: Option<PaneId>,
}

pub enum Node {
    Split { axis: Axis, ratio: f32, a: Box<Node>, b: Box<Node> },
    Leaf(PaneId),
}

pub struct Pane {
    pub id: PaneId,
    pub kind: PaneKind,
    pub settings: PaneSettings,
}
```

`Layout` is the **single source of truth**, serialized directly. No
`Configuration` ↔ `LayoutNode` conversion pair; `pane_grid::State` is rebuilt
from it when it changes, never the reverse.

## Layer 2 — Pane content as an enum, not a trait

```rust
pub enum PaneKind {
    Library,
    Queue,
    Collections,
    NowPlaying,
    Artwork,
    Spectrum,
    VuMeters,
    Timeline,
    Empty,
}
```

Every pane's state lives in one `PaneState` enum with a variant per kind:

```rust
pub enum PaneState {
    Library(library::State),
    Collections(collections::State),
    Queue(queue::State),
    ...
}
```

**This is the fix for causes 1–3.** Addressing a pane becomes:

```rust
if let Some(PaneState::Collections(state)) = self.pane_state_mut(id) { ... }
```

No `Any`, no `downcast`, no broadcast loop — and adding a pane kind produces a
non-exhaustive-match error at every site that must handle it.

## Layer 3 — Per-pane messages

```rust
pub enum Message {
    Pane(PaneId, PaneMessage),
    Layout(LayoutMessage),
    ...
}

pub enum PaneMessage {
    Library(library::Message),
    Collections(collections::Message),
    ...
}
```

Messages carry the `PaneId` they belong to, so `update` routes to exactly one
pane. Two Collections panes stay independent — which the old system could not do.

## Sizing

Each pane declares how it wants space:

```rust
pub enum Sizing {
    Fill,
    Fixed(f32),
    Auto,
}
```

`Fill` panes share what is left after `Fixed` panes take their pixels. `Auto`
means "as tall as the content needs" — the natural fit for a transport bar or a
timeline.

How this is implemented depends on the decision below.

## Pane settings

Per-kind settings, edited in a popover from the pane's own header rather than a
global preferences dialog:

```rust
pub struct PaneSettings {
    pub sizing: Sizing,
    pub show_header: bool,
    pub kind: KindSettings,
}

pub enum KindSettings {
    Library { columns: Vec<Column>, sort: Sort, group_by: Option<GroupBy> },
    Spectrum { bar_count: usize, style: SpectrumStyle, smoothing: f32 },
    Artwork { fit: Fit, show_reflection: bool },
    ...
}
```

Serialized with the layout, so a saved layout captures both arrangement *and*
configuration.

## Persistence

One file, one representation:

```toml
[[layouts]]
name = "Default"

[layouts.root]
type = "split"
axis = "vertical"
ratio = 0.25
  [layouts.root.a]
  type = "leaf"
  kind = "collections"
  sizing = { type = "fixed", pixels = 260 }
  [layouts.root.b]
  type = "leaf"
  kind = "library"
  sizing = "fill"
```

Multiple named layouts, switchable by number key — keeping the feature from the
old system without the three-representation problem.

## Edit mode

Preserved from the old system, but pane-local rather than global:

- Drag a pane's header to move it; drop targets highlight.
- Drag a divider to resize.
- A pane's header menu offers: change kind, split horizontally/vertically,
  configure, close.
- Outside edit mode, headers hide and dividers stop responding, so ordinary use
  cannot disturb the layout.

---

# The fixed-size decision

Three ways to get fixed-size panes given `pane_grid`'s ratio-only splits.

### Option A — Shell + pane_grid

Fixed regions live **outside** the grid, in a plain `row!`/`column!` with
explicit `Length::Fixed`. `pane_grid` fills the remaining centre area.

```
┌─────────────────────────────────────┐
│  header (Auto)                      │
├────────┬────────────────────────────┤
│sidebar │                            │
│(Fixed) │   pane_grid (Fill)         │
│        │   user-splittable          │
├────────┴────────────────────────────┤
│  transport bar (Fixed)              │
└─────────────────────────────────────┘
```

- **For:** uses `pane_grid` as designed; no custom layout code; resize/drag/split
  all work inside the grid for free.
- **Against:** fixed regions are not user-repositionable. The sidebar cannot be
  dragged into the centre.

### Option B — Custom layout widget

Implement `Widget` directly: walk the `Node` tree, give `Fixed` children their
pixels, distribute the remainder among `Fill` children.

- **For:** exactly the requested model — any pane fixed or filling, anywhere.
  Per-pane min/max. Pixel splits that survive window resize.
- **Against:** reimplements drag, drop-target highlighting, divider hit-testing,
  and keyboard focus. Roughly 600–900 lines, and the drag/drop interaction is
  the fiddly part.

### Option C — pane_grid + ratio correction

Keep `pane_grid`, but on every window resize recompute the ratios so `Fixed`
panes hold their pixel size.

- **For:** small change; keeps drag/drop.
- **Against:** fights the widget. Ratios are recomputed after layout, so panes
  visibly jump during a resize drag, and rounding drifts. **Not recommended** —
  it will feel wrong in exactly the situation it exists to handle.

---

# Recommendation

**Start with A, keep the door open to B.**

Because layout is plain data (Layer 1) and content is an enum (Layer 2), the
rendering backend is swappable. Option A ships a working, pleasant system
quickly; if fixed panes must become draggable later, only the rendering layer
changes — `Layout`, `PaneKind`, `PaneState`, messages, and the config format all
stay.

Doing B first means writing a custom widget before knowing which interactions
actually matter.

---

# Sequencing

1. `layout.rs` — `Layout`, `Node`, `PaneId`, `Sizing`, serde. No iced types.
2. `pane/mod.rs` — `PaneKind`, `PaneState`, `PaneMessage`, per-kind modules.
3. Render via Option A; wire `pane_grid` for the centre region.
4. Edit mode — header menus, split, close, change kind.
5. Per-pane settings popovers.
6. Named layout presets and persistence.

Steps 1–2 are the ones that fix the old design's problems; 3–6 are features on
top of them.

## Open questions

- **Header style.** Always-visible title bars, or headers only in edit mode?
  Affects how much vertical space small panes lose.
- **Tabs.** Should a pane hold multiple kinds as tabs, like Halloy's buffers?
  Powerful, but a significant addition to the model.
- **Presets.** Keep number-key switching from the old system, or a single
  layout that is simply always saved?
