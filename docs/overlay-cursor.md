# Overlays and the falling-through cursor

A bug that affects any iced application with a custom `Overlay`, written to be
portable: nothing here is specific to this project. Verified against iced
0.14.

## The symptom

An overlay — a dropdown panel, a context menu, a popover — is open and visibly
covering the content beneath it. The cursor moves over a part of the panel that
is not itself interactive: its padding, the gap between two items, a separator,
an empty region below the last row.

The content *behind* the overlay reacts. A list highlights the row under the
cursor. A button lights up. Hover tooltips appear. All of it through a panel
that is plainly drawn on top.

Clicking usually still behaves, because the overlay captures the press. Only
hover leaks, which is what makes this look cosmetic and easy to misfile as a
z-order or clipping problem. It is neither.

## The cause

iced does not decide overlay hit-testing geometrically. It asks the overlay.

From `iced_runtime`'s `user_interface.rs`, in the update path:

```rust
let interaction = overlay.mouse_interaction(
    Layout::new(&layout),
    mouse::Cursor::Available(cursor_position),
    renderer,
);

if interaction == mouse::Interaction::None {
    (cursor, mouse::Interaction::None)          // base layer keeps the real cursor
} else {
    (mouse::Cursor::Unavailable, interaction)   // base layer is blinded
}
```

The base layer is only blinded when the overlay's `mouse_interaction` returns
something other than `Interaction::None`. That return value is doing two
unrelated jobs at once: it picks the cursor *icon*, and it declares whether the
overlay *claims the cursor at all*.

A typical `mouse_interaction` delegates to its content, and the content answers
per-widget. Buttons return `Pointer`. Text returns `Text`. Padding, spacing, and
containers return `None`, because they have no opinion about the cursor's
appearance. That `None` is then read by `user_interface` as "this overlay does
not want the cursor here" — and the cursor is handed straight back to whatever
is underneath.

So the rule is: **any pixel of your overlay whose content has no cursor opinion
is a hole the cursor falls through.**

## The fix

Claim the cursor whenever it is over the overlay, regardless of what the content
thinks the icon should be. `Interaction::Idle` is the plain arrow — it claims the
cursor without changing how it looks, which is exactly the neutral answer the
enum otherwise cannot express:

```rust
fn mouse_interaction(
    &self,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    renderer: &Renderer,
) -> mouse::Interaction {
    let viewport = layout.bounds();
    let interaction = self.content.as_widget().mouse_interaction(
        &self.content_tree, layout, cursor, &viewport, renderer,
    );

    if interaction == mouse::Interaction::None && cursor.is_over(viewport) {
        return mouse::Interaction::Idle;
    }

    interaction
}
```

Do not substitute `Pointer` here. It claims the cursor correctly but tells the
user every dead pixel is clickable.

## The second half: nested overlays

The fix above is not sufficient once an overlay can open an overlay of its own —
a dropdown whose items have fly-out submenus, a menu with a nested picker.

From `iced_core`'s `overlay/nested.rs`:

```rust
overlay
    .overlay(layout, renderer)
    .zip(layouts.next())
    .and_then(|(mut overlay, layout)| recurse(&mut overlay, layout, cursor, renderer))
    .unwrap_or_else(|| overlay.mouse_interaction(layout, cursor, renderer))
```

When a child overlay exists, **only the child is consulted.** The parent's
`mouse_interaction` is not called at all — `unwrap_or_else` is the no-child
branch.

The consequence is precise and easy to misread as intermittent. With no submenu
open, the parent panel answers for itself and behaves. The instant a submenu
opens, the parent stops being asked; the child correctly answers `None` for a
cursor outside its own bounds; and nothing is left claiming the parent panel.
Hover starts falling through the parent — but only while a submenu happens to be
open, which is why this presents as "sometimes".

So a nested overlay must answer for its ancestors too. Give the child the
parent's rectangle and check both:

```rust
struct FlyOutOverlay<'a, 'b, Message> {
    // ...
    /// The panel this fly-out belongs to. While a fly-out is open, iced's
    /// `overlay::Nested` asks only the innermost overlay what the cursor is
    /// doing, so this overlay answers for its parent panel too.
    panel_bounds: Rectangle,
}

if interaction == mouse::Interaction::None
    && (cursor.is_over(viewport) || cursor.is_over(self.panel_bounds))
{
    return mouse::Interaction::Idle;
}
```

The parent's rectangle is usually already to hand: the `viewport` argument that
the parent passes into its content's `overlay()` is the parent panel's bounds.
Translate it by the same `translation` the child receives.

## Applying this to a project

1. Every custom `Overlay` implementation needs the `Idle` fallback. There is no
   shared base class to put it in, so it is repeated per overlay — comment it at
   each site rather than leaving it looking like a stray special case.
2. Any overlay that can host a *nested* overlay additionally needs the ancestor
   check, and the nested child is where that code goes, not the parent.
3. Depth beyond two levels compounds: each level must claim every ancestor above
   it, since only the innermost is ever asked.

## What does not need this

`stack!` layers are fine. `iced_widget`'s `stack` levitates the cursor for lower
children itself:

```rust
if i < end && is_over && !cursor.is_levitating() {
    let interaction = child.as_widget().mouse_interaction(...);
    if interaction != mouse::Interaction::None {
        cursor = cursor.levitate();
    }
}
```

Note it has the same `!= None` condition, so a stack layer with dead space
leaks too — but `Cursor::Levitating` makes `position()` return `None`, so any
widget below that hit-tests with `cursor.position_over(bounds)` correctly sees
nothing. Widgets that hit-test that way are already safe; widgets that reach for
raw coordinates are not.

## Why this is hard to find

- It looks like a z-order or clip-bounds bug, so the search starts in layout code.
- Clicks work, so it reads as cosmetic.
- With nesting it is intermittent, which suggests a state bug rather than a
  structural one.
- The mechanism is a return value doing double duty — icon *and* ownership — and
  nothing in the type names says so.
