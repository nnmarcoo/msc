# Pane system

The GUI's layout: a binary tree of panes the user builds by splitting, dragging,
resizing, and locking, with a clean/edit-mode duality. Replaces the earlier
`pane_grid` approach (see `layout.md`), which could not express fixed-size panes.

This document reflects what is built, not a plan. Where the implementation
diverged from the original design, the divergence and its reason are noted.

## Status

Working and in use:

- Binary-tree layout, rendered by composition (not a custom widget).
- Clean/edit mode duality; edit controls overlay content without displacing it.
- Split, close, change-kind, per-pane lock — from an edit-mode control cluster.
- Draggable dividers (delta-based) with a resize cursor.
- Pane drag-and-drop: onto a pane's edge to split, its centre to swap, or a
  window edge to wrap the whole layout.
- Named layout presets, number-key switching, persistence to config.

Deferred: real pane content (panes render as centred kind labels), the
context-menu edit entry, and per-pane settings beyond the lock.

## The two modes

**Normal mode** renders *only pane content* — no chrome, no handles. It looks
like a finished application. Entered/left by the `e` key.

**Edit mode** overlays editing affordances on top of the live content *without
displacing it*, so the user edits a true preview. This is the load-bearing
constraint: an early version stacked a header *above* the content, which shrank
it and defeated the preview. Controls are instead an overlay — content on layer
0, controls on layer 1 via `stack!` — so leaving edit mode drops the overlay and
changes nothing beneath it.

> **Divergence from the design.** The plan called for a corner button opening a
> popover. What shipped is a small always-visible control cluster in each pane's
> top-right corner (lock, split ↔, split ↕, change-kind, close, and a `⠿` drag
> handle). A popover can still replace it later; the cluster was faster to reach
> a working edit mode.

## The sizing model

A locked pane behaves like the VSCode sidebar:

- **Locked** → holds its pixel size when the window or its siblings grow.
- **Fill** siblings absorb all the resize.
- When the window shrinks below what fits, the locked pane **compresses too**
  (down to `MIN_PANE`) rather than pushing content off-screen.
- In edit mode, dragging a locked pane's divider **sets** its locked size.

This is exactly a flex layout: a fixed-basis child that does not grow but yields
under pressure, beside fill children that take the slack.

### Why `pane_grid` cannot do it

`pane_grid` stores each split as a single `ratio`; pixel sizes are *derived* from
`ratio × available_space` every layout pass (`pane_grid/node.rs::compute_regions`).
A "locked" pane would still scale with the window — the opposite of the
requirement. `min_size` is one global value for the whole grid, and there is no
`max_size`. There is nowhere to store a pixel constraint that survives a resize.

### How it is done instead: composition, not a custom widget

> **The biggest divergence from the design.** The plan was to hand-write a custom
> `Widget` that walks the tree and lays panes out itself (~600–900 lines). That
> proved unnecessary.

Each `Split` renders as a plain `row!`/`column!` whose first child gets a
`Length` — `Length::Fixed(pixels)` for a lock, `Length::FillPortion` for a ratio
— and whose second child fills. iced's own flex layout then delivers the VSCode
rule *for free*, verified from its source: a `Fixed` child is laid out against
the *remaining* available space (`iced_core .../flex.rs`), so it holds its pixels
while fill siblings absorb resize, and clamps down only when the container
underflows.

The renderer (`app/render.rs`) is ~90 lines. If dragging or overlays ever need
control this can't give, dropping to a custom widget is still open — the data
model is renderer-agnostic by design, so only the renderer would change.

## Data model (`layout.rs`)

Layout is plain data, free of iced types, serialised directly. The rendered
widget is rebuilt from it every frame, never the reverse, so there is only ever
one representation to keep correct.

```rust
pub enum Node {
    Leaf { id: PaneId },
    Split { axis: Axis, split: Split, a: Box<Node>, b: Box<Node> },
}

pub enum Split {
    Ratio  { ratio: f32 },              // proportional, both flex
    Locked { side: Side, pixels: f32 }, // one side fixed, the other fills
}
```

> **Divergence from the design.** The plan modelled sizing as "how child `a` is
> sized," with `b` always the remainder. That cannot express locking the
> *second* child to N pixels without knowing the total width. The shipped model
> names the locked `Side` explicitly (`A` or `B`), so either child can be the
> locked one. This was a bug caught mid-implementation, not a preference.

`MIN_PANE = 80.0` bounds compression so a shrinking window cannot collapse a pane
to nothing. Pane *kinds* live in a parallel `Vec<PaneEntry>` keyed by `PaneId`,
kept in step with the tree by `reconcile()`.

### Key operations

- `split(target, axis, kind)` — replaces a leaf with a split of it and a new pane.
- `close(target)` — removes a pane, collapsing its split; refuses the last one.
- `set_lock(id, Option<pixels>)` — locks/unlocks the pane's side of its split.
- `drag_divider(path, delta, span)` — resizes from a cursor delta (below).
- `move_pane(source, target, zone)` — drag-drop against a pane (below).
- `move_pane_to_root_edge(source, edge)` — drag-drop against the window (below).

## Dividers (delta-based)

In edit mode a thin draggable seam sits between a split's two children, showing a
resize cursor on hover (`ResizingHorizontally` / `Vertically`).

> **Divergence from the design.** The plan assumed the renderer would know each
> split's on-screen rectangle. Composition does not expose rectangles, so
> dividers are **delta-based** instead: a press starts the drag, the global
> cursor stream feeds pixel deltas, release commits.

- A **locked** split maps the delta 1:1 to pixels — exact.
- A **ratio** split converts the delta over a `span`. `span` is the whole-window
  dimension along the axis: exact for top-level splits, an approximation for
  nested ones (a nested ratio drag feels roughly 2× fast). Locked drags are
  unaffected. Refinable later by measuring regions if it bothers.

A split is addressed by `SplitPath` — the sequence of `Side` turns from the root
— which is stable within a layout and cheap to compare.

### Cursor during a fast drag

While a divider is dragged, the cursor can outrun the thin seam and land over
pane content, which would reset the cursor. Fixed by wrapping the whole layout in
a `mouse_area` that forces the resize cursor *for the duration of the drag*.
`mouse_area` only applies its interaction when the content underneath requests
none, so it never fights the seam's own hover cursor.

## Pane drag-and-drop

Grab a pane's `⠿` handle to start a drag. Every other pane becomes a hover
target; a highlight previews where a drop would land; release commits.

### Finding the drop target without rectangles

`pane_grid`'s drag is a custom widget that owns every pane's rectangle and
hit-tests against them — not reusable here. Instead each pane wraps its content
in `responsive` (which hands the closure the pane's real size) and a `mouse_area`
whose `on_move` reports the cursor's fractional position. `DropZone::from_fraction`
maps that to a zone: outer third of an edge → split that side; centre third →
swap.

### The drop-preview highlight

The target pane paints a translucent overlay over the half a drop would occupy
(left/right/top/bottom) or the whole pane (centre swap).

> **Bug fixed here.** The first version sized the highlight with a single
> `FillPortion(50)` child, which fills 100% because `FillPortion` only divides
> space *between siblings* — with no sibling it takes everything. Every zone lit
> the whole pane. Fixed by making the highlight a real two-child flex (lit half +
> empty spacer), so the split is actually previewed.

### Whole-window edge drops

Dropping against the *outer* edge of the window wraps the entire layout in a new
split — e.g. drop a pane at the very bottom and it spans the full width beneath
everything, not just under the pane the cursor happens to be over.

Implemented as a ~24px band along each window edge, overlaid on the pane sensors
during a drag. `move_pane_to_root_edge` detaches the pane and wraps the whole
remaining tree.

> **Bug fixed here.** iced's `mouse_area` does **not** capture the event on
> `on_enter`/`on_move`/`on_exit` (only on `on_press`). So near a border, both the
> edge band's `on_enter` and the pane sensor's continuous `on_move` fired every
> frame, and the pane message — arriving second — clobbered the edge every time.
> Only pane splits/swaps were ever visible. Fixed by tracking the root edge and
> the pane zone as **separate** fields on the drag state, with the root edge
> always winning (`PaneDrag::over`). The pane's `on_move` can no longer overwrite
> an active edge.

## Rendering pipeline

```
Layout (data)
   │  app/render.rs   walks the Node tree
   ▼
row! / column! nesting        each Split → flex row/column
   │  Fixed | FillPortion     per-side Length from the Split
   ▼
pane/view.rs                  per pane: content (+ edit overlay in edit mode)
   │  stack!                  content · drop highlight · hover sensor / controls
   ▼
root_edge_band                during a drag: window-edge sensors + highlight
```

Layout mutations flow the other way as messages (`app/mod.rs`): grab/hover/drop
and divider drags update the `Layout`, which is then re-rendered and persisted.

## Files

- `layout.rs` — the data model, tree ops, and unit tests (14).
- `app/render.rs` — tree → nested flex widgets; per-side `Length`.
- `app/mod.rs` — drag state (`DividerDrag`, `PaneDrag`), messages, update/view.
- `pane/view.rs` — per-pane rendering, edit overlay, hover sensors, edge band.
- `pane/mod.rs` — `PaneKind`, `PaneState`, `PaneStates` (content, mostly deferred).

## Tests

14 unit tests in `layout.rs`, run with `cargo test --bin verse` (the GUI is a
bin, not a lib). They cover the mutation logic that a headless run can verify:
divider drag direction for both locked sides, ratio conversion, `MIN_PANE`
clamping, lock persistence round-trip, drop-zone geometry, edge-split placement,
centre swap, self-drop no-op, tree validity after a move, and whole-window edge
wrapping. Interaction *feel* (cursor, drag latency, highlight) is not unit-
testable and was confirmed by hand.

## Deferred

- **Pane content** — panes show centred kind labels; the `PaneState` machinery
  exists but is not driven.
- **Context-menu edit entry** — currently `e` only; empty-space right-click planned.
- **Per-pane settings** beyond the lock (the plan's `KindSettings`).
- **Nested-split ratio drag speed** — see Dividers.
- **Custom widget** — remains an option if composition ever falls short; the data
  model would not change.
