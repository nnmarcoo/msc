//! A virtualised list of queued tracks, reorderable by dragging.
//!
//! A custom widget partly because a row is not worth an `Element`, as in
//! [`crate::widgets::track_list`], but mostly because of the cursor. The queue was
//! once a `column` of per-row `mouse_area`s, and that arrangement produced three
//! bugs of one shape: rows were independent widgets whose hover messages raced.
//!
//! Moving the pointer up the list lost the highlight, because iced delivers a
//! departure and an arrival in layout order rather than in the order they
//! happened, so the upper row's arrival landed first and the lower row's departure
//! cleared it. Moving between two copies of one track lost it too, since a track id
//! cannot tell "the pointer left me" from "the pointer moved to another copy of
//! me". Removing a track left nothing hovered, because the rows below shifted up
//! under a stationary cursor and a row already hovered fires no `on_enter`.
//!
//! One widget keeps one `hovered` row, recomputed from the cursor by
//! [`RowMetrics::row_at`] on every event. That makes all three unrepresentable
//! rather than fixed: nothing races, the row is an index and so unique, and a list
//! that changes shape is re-measured on the next event like any other.
//!
//! Virtualisation works as [`crate::widgets::track_list`] does. The `viewport`
//! iced passes to `draw` is the visible slice in content coordinates, so the rows
//! worth drawing are two divisions.
//!
//! # Dragging
//!
//! A press on a row's grip arms a drag and a move past [`DRAG_THRESHOLD`] starts
//! it. Only the grip, so the rest of the row keeps its clicks, and the threshold
//! stops a press that never travels from reordering anything.
//!
//! The drag lives in widget state and the press captures the pointer, so it keeps
//! tracking wherever the cursor goes and a release anywhere commits. This is the
//! other thing one widget buys. The old arrangement had to route moves through the
//! app, since a per-row `mouse_area` stops hearing about the pointer once it
//! leaves that row, which took two message kinds and a pane-level row key.
//!
//! `drop_at` counts gaps rather than rows, so it runs one past the last row:
//! dropping below everything is a real destination no row index can name.
//! [`drop_target`] turns a gap into a destination, which is not the identity,
//! because the queue lifts the dragged track out before inserting it and so every
//! gap below the lifted row means a position one lower.
//!
//! # What the widget does not decide
//!
//! Hover is published as well as kept, since other panes highlight the same track.
//! Selection and the queue itself live on the app. The widget reports what the user
//! did as an [`Op`] and mutates nothing, so a reorder here and one from a future
//! keyboard shortcut cannot disagree.
//!
//! Rows draw in layers, one each for playing, selected, hovered and dragging,
//! because a row can be several at once and a single ranked state would have to
//! choose between them.

use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::Renderer as _;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, svg, text};
use iced::alignment::Vertical;
use iced::widget::svg::Handle;
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme,
    mouse,
};

use std::sync::LazyLock;

use crate::styles::radius;
use crate::tracks::{Context, QueueRow, Slot};

const ICON_CLOSE: &[u8] = include_bytes!("../../../assets/icons/close.svg");
const ICON_PLAYING: &[u8] = include_bytes!("../../../assets/icons/play.svg");
const ICON_GRIP: &[u8] = include_bytes!("../../../assets/icons/grip.svg");

static CLOSE: LazyLock<Handle> = LazyLock::new(|| Handle::from_memory(ICON_CLOSE));
static PLAYING: LazyLock<Handle> = LazyLock::new(|| Handle::from_memory(ICON_PLAYING));
static GRIP: LazyLock<Handle> = LazyLock::new(|| Handle::from_memory(ICON_GRIP));

pub const ROW_HEIGHT: f32 = 46.0;

const PAD_H: f32 = 10.0;
const TITLE_SIZE: f32 = 13.0;
const LABEL_SIZE: f32 = 11.0;
const LINE_GAP: f32 = 3.0;

const POSITION_WIDTH: f32 = 26.0;
const DURATION_WIDTH: f32 = 38.0;
const REMOVE_WIDTH: f32 = 24.0;
const COLUMN_GAP: f32 = 10.0;

const GRIP_ICON: f32 = 13.0;
const PLAYING_ICON: f32 = 9.0;
const REMOVE_ICON: f32 = 14.0;
const INDICATOR_HEIGHT: f32 = 2.0;

/// How far a press on the grip must travel before it is a drag.
const DRAG_THRESHOLD: f32 = 4.0;

const _: () = {
    assert!(GRIP_ICON <= POSITION_WIDTH);
    assert!(PLAYING_ICON <= POSITION_WIDTH);
    assert!(REMOVE_ICON <= REMOVE_WIDTH);
    assert!(ROW_HEIGHT > crate::widgets::track_list::ROW_HEIGHT);
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// The row under the cursor changed. Carries the track so other panes can
    /// highlight it, and `None` once the pointer leaves the list.
    Hovered(Option<i64>),
    /// A double-click on a row: play it now.
    Activated(i64),
    /// The remove control on the row at this queue position was pressed.
    Removed(usize),
    /// A drag committed, moving the track at `from` to `to`. Both index the
    /// queue's upcoming deque, and `to` is where it lands once lifted out.
    Reordered { from: usize, to: usize },
}

#[derive(Default)]
struct State {
    hovered: Option<usize>,
    armed: Option<Armed>,
    dragging: Option<usize>,
    drop_at: Option<usize>,
    last_press: Option<(usize, std::time::Instant)>,
}

/// A press on a grip that has not yet travelled far enough to be a drag.
#[derive(Debug, Clone, Copy)]
struct Armed {
    upcoming: usize,
    origin: Point,
}

const DOUBLE_CLICK: std::time::Duration = std::time::Duration::from_millis(400);

pub struct QueueList<'a, Message> {
    rows: Vec<QueueRow<'a>>,
    context: Context<'a>,
    on_op: Box<dyn Fn(Op) -> Message + 'a>,
}

impl<'a, Message> QueueList<'a, Message> {
    pub fn new(
        rows: Vec<QueueRow<'a>>,
        context: Context<'a>,
        on_op: impl Fn(Op) -> Message + 'a,
    ) -> Self {
        Self {
            rows,
            context,
            on_op: Box::new(on_op),
        }
    }

    fn metrics(&self, bounds: Rectangle) -> RowMetrics {
        RowMetrics {
            bounds,
            len: self.rows.len(),
        }
    }

    fn visible_range(&self, bounds: Rectangle, viewport: &Rectangle) -> std::ops::Range<usize> {
        visible_range(bounds, viewport, self.rows.len())
    }

    /// The queue position of the row at `index`, if it has one. History and the
    /// playing track do not, and so can be neither removed nor dragged.
    fn upcoming_at(&self, index: usize) -> Option<usize> {
        self.rows.get(index).and_then(|row| row.upcoming)
    }
}

/// The geometry of the list, and every question that is asked of it.
///
/// Split out so the arithmetic can be tested without building a widget or a
/// `Library`; the bugs this widget exists to prevent were all in this mapping.
#[derive(Debug, Clone, Copy)]
struct RowMetrics {
    bounds: Rectangle,
    len: usize,
}

impl RowMetrics {
    fn row_at(self, cursor: mouse::Cursor) -> Option<usize> {
        let position = cursor.position_over(self.bounds)?;
        self.row_at_y(position.y)
    }

    fn row_at_y(self, y: f32) -> Option<usize> {
        let index = ((y - self.bounds.y) / ROW_HEIGHT).floor();
        if index < 0.0 {
            return None;
        }
        let index = index as usize;
        (index < self.len).then_some(index)
    }

    fn row_bounds(self, index: usize) -> Rectangle {
        Rectangle {
            x: self.bounds.x,
            y: self.bounds.y + index as f32 * ROW_HEIGHT,
            width: self.bounds.width,
            height: ROW_HEIGHT,
        }
    }

    /// The grip's hit region: the position column of a row.
    fn grip_bounds(self, index: usize) -> Rectangle {
        let row = self.row_bounds(index);
        Rectangle {
            x: row.x + PAD_H,
            y: row.y,
            width: POSITION_WIDTH,
            height: row.height,
        }
    }

    /// The remove control's hit region, at the row's trailing edge.
    fn remove_bounds(self, index: usize) -> Rectangle {
        let row = self.row_bounds(index);
        Rectangle {
            x: row.x + row.width - PAD_H - REMOVE_WIDTH,
            y: row.y + (row.height - REMOVE_WIDTH) / 2.0,
            width: REMOVE_WIDTH,
            height: REMOVE_WIDTH,
        }
    }

    /// Which gap a cursor at `y` is aiming at, counted in gaps so the value one
    /// past the last row means "below everything".
    ///
    /// A row's upper half means the gap above it and its lower half the gap
    /// below, so every pixel belongs to some gap and the marker never disappears
    /// mid-drag. Past the end of the list the last gap holds, rather than the
    /// target snapping back to the row under a cursor that has run out of rows.
    fn gap_at_y(self, y: f32) -> usize {
        let offset = ((y - self.bounds.y) / ROW_HEIGHT).max(0.0);
        let index = offset.floor() as usize;
        if index >= self.len {
            return self.len;
        }
        if offset - index as f32 > 0.5 {
            index + 1
        } else {
            index
        }
    }
}

fn visible_range(bounds: Rectangle, viewport: &Rectangle, len: usize) -> std::ops::Range<usize> {
    let Some(visible) = bounds.intersection(viewport) else {
        return 0..0;
    };
    let first = (((visible.y - bounds.y) / ROW_HEIGHT).floor().max(0.0) as usize).min(len);
    let count = (visible.height / ROW_HEIGHT).ceil() as usize + 1;
    first..first.saturating_add(count).min(len)
}

/// Where a drag from `from` landing in gap `gap` puts the track.
///
/// Returns `None` when the drop would not move anything, so a caller can skip a
/// no-op rather than having to recognise one.
pub fn drop_target(from: usize, gap: usize) -> Option<usize> {
    let to = if gap > from { gap - 1 } else { gap };
    (to != from).then_some(to)
}

impl<Message> Widget<Message, Theme, Renderer> for QueueList<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let height = (self.rows.len() as f32 * ROW_HEIGHT).max(limits.min().height);
        layout::Node::new(Size::new(limits.max().width, height))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let metrics = self.metrics(layout.bounds());
        let state = tree.state.downcast_mut::<State>();

        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::WheelScrolled { .. }) => {
                let Some(at) = cursor.position() else {
                    return;
                };

                if let Some(armed) = state.armed
                    && state.dragging.is_none()
                    && armed.origin.distance(at) > DRAG_THRESHOLD
                {
                    state.dragging = Some(armed.upcoming);
                }

                if state.dragging.is_some() {
                    let gap = metrics.gap_at_y(at.y);
                    if state.drop_at != Some(gap) {
                        state.drop_at = Some(gap);
                        shell.request_redraw();
                    }
                    return;
                }

                let row = metrics.row_at(cursor);
                if row != state.hovered {
                    state.hovered = row;
                    let id = row
                        .and_then(|index| self.rows.get(index))
                        .and_then(|r| r.track.id());
                    shell.publish((self.on_op)(Op::Hovered(id)));
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorLeft)
                if state.dragging.is_none() && state.hovered.take().is_some() =>
            {
                shell.publish((self.on_op)(Op::Hovered(None)));
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(index) = metrics.row_at(cursor) else {
                    return;
                };

                if let Some(position) = self.upcoming_at(index) {
                    if cursor.is_over(metrics.remove_bounds(index)) {
                        shell.publish((self.on_op)(Op::Removed(position)));
                        shell.capture_event();
                        shell.request_redraw();
                        return;
                    }
                    if cursor.is_over(metrics.grip_bounds(index))
                        && let Some(at) = cursor.position()
                    {
                        state.armed = Some(Armed {
                            upcoming: position,
                            origin: at,
                        });
                        shell.capture_event();
                        return;
                    }
                }

                let now = std::time::Instant::now();
                let repeat = state
                    .last_press
                    .is_some_and(|(last, at)| last == index && now - at < DOUBLE_CLICK);

                if repeat {
                    state.last_press = None;
                    if let Some(id) = self.rows.get(index).and_then(|r| r.track.id()) {
                        shell.publish((self.on_op)(Op::Activated(id)));
                        shell.capture_event();
                    }
                } else {
                    state.last_press = Some((index, now));
                }
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let committed = state
                    .dragging
                    .zip(state.drop_at)
                    .and_then(|(from, gap)| drop_target(from, gap).map(|to| (from, to)));

                let was_dragging = state.dragging.is_some();
                state.armed = None;
                state.dragging = None;
                state.drop_at = None;

                if let Some((from, to)) = committed {
                    shell.publish((self.on_op)(Op::Reordered { from, to }));
                }
                if was_dragging {
                    state.hovered = metrics.row_at(cursor);
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let metrics = self.metrics(bounds);
        let palette = theme.extended_palette();
        let state = tree.state.downcast_ref::<State>();

        for index in self.visible_range(bounds, viewport) {
            let Some(entry) = self.rows.get(index) else {
                continue;
            };
            let row = metrics.row_bounds(index);
            let hovered = state.hovered == Some(index);
            let dragging = entry.upcoming.is_some() && state.dragging == entry.upcoming;

            let row_state = entry
                .track
                .id()
                .map(|id| self.context.row_state(id))
                .unwrap_or_default();

            if row_state.selected {
                fill_row(renderer, row, palette.primary.base.color.scale_alpha(0.30));
            }
            if entry.slot == Slot::Current {
                fill_row(renderer, row, palette.primary.base.color.scale_alpha(0.18));
            }
            if row_state.hovered || hovered {
                fill_row(
                    renderer,
                    row,
                    palette.background.strong.color.scale_alpha(0.55),
                );
            }
            if dragging {
                fill_row(
                    renderer,
                    row,
                    palette.background.strong.color.scale_alpha(0.9),
                );
            }

            draw_row(renderer, palette, metrics, index, entry, hovered);
        }

        if let Some(gap) = state.drop_at.filter(|_| state.dragging.is_some()) {
            let y = bounds.y + (gap.min(self.rows.len()) as f32) * ROW_HEIGHT;
            fill(
                renderer,
                Rectangle {
                    x: bounds.x + PAD_H,
                    y: y - INDICATOR_HEIGHT / 2.0,
                    width: (bounds.width - PAD_H * 2.0).max(0.0),
                    height: INDICATOR_HEIGHT,
                },
                palette.primary.base.color,
                INDICATOR_HEIGHT / 2.0,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        if state.dragging.is_some() {
            return mouse::Interaction::Grabbing;
        }

        let metrics = self.metrics(layout.bounds());
        let Some(index) = metrics.row_at(cursor) else {
            return mouse::Interaction::default();
        };

        if self.upcoming_at(index).is_some() {
            if cursor.is_over(metrics.grip_bounds(index)) {
                return mouse::Interaction::Grab;
            }
            if cursor.is_over(metrics.remove_bounds(index)) {
                return mouse::Interaction::Pointer;
            }
        }
        mouse::Interaction::Pointer
    }
}

fn draw_row(
    renderer: &mut Renderer,
    palette: &iced::theme::palette::Extended,
    metrics: RowMetrics,
    index: usize,
    entry: &QueueRow<'_>,
    hovered: bool,
) {
    let row = metrics.row_bounds(index);
    let dimmed = palette.background.base.text.scale_alpha(0.6);

    let text_color = match entry.slot {
        Slot::Played => palette.background.base.text.scale_alpha(0.45),
        Slot::Current => palette.primary.base.color,
        Slot::Upcoming => palette.background.base.text,
    };

    let slot = metrics.grip_bounds(index);
    match (entry.slot, entry.upcoming) {
        (Slot::Upcoming, Some(_)) if hovered => {
            draw_icon(renderer, &GRIP, slot, GRIP_ICON, dimmed);
        }
        (Slot::Upcoming, Some(position)) => {
            draw_text(
                renderer,
                &format!("{}", position + 1),
                slot,
                LABEL_SIZE,
                dimmed,
                text::Alignment::Center,
            );
        }
        (Slot::Current, _) => {
            draw_icon(
                renderer,
                &PLAYING,
                slot,
                PLAYING_ICON,
                palette.primary.base.color,
            );
        }
        _ => {}
    }

    let removable = entry.upcoming.is_some() && hovered;
    let trailing = if removable {
        REMOVE_WIDTH + COLUMN_GAP
    } else {
        0.0
    };

    let text_x = slot.x + POSITION_WIDTH + COLUMN_GAP;
    let text_width =
        (row.x + row.width - PAD_H - trailing - DURATION_WIDTH - COLUMN_GAP - text_x).max(0.0);

    let (title_y, artist_y) = two_lines(row);

    draw_text(
        renderer,
        entry.track.title().unwrap_or("\u{2014}"),
        Rectangle {
            x: text_x,
            y: title_y,
            width: text_width,
            height: TITLE_SIZE,
        },
        TITLE_SIZE,
        text_color,
        text::Alignment::Left,
    );
    draw_text(
        renderer,
        entry.track.track_artist().unwrap_or("\u{2014}"),
        Rectangle {
            x: text_x,
            y: artist_y,
            width: text_width,
            height: LABEL_SIZE,
        },
        LABEL_SIZE,
        dimmed,
        text::Alignment::Left,
    );

    draw_text(
        renderer,
        &clock(entry.track.duration()),
        Rectangle {
            x: text_x + text_width + COLUMN_GAP,
            y: row.y,
            width: DURATION_WIDTH,
            height: row.height,
        },
        LABEL_SIZE,
        dimmed,
        text::Alignment::Right,
    );

    if removable {
        draw_icon(
            renderer,
            &CLOSE,
            metrics.remove_bounds(index),
            REMOVE_ICON,
            dimmed,
        );
    }
}

/// The two text baselines of a row, centred as a pair.
fn two_lines(row: Rectangle) -> (f32, f32) {
    let block = TITLE_SIZE + LINE_GAP + LABEL_SIZE;
    let top = row.y + (row.height - block) / 2.0;
    (top, top + TITLE_SIZE + LINE_GAP)
}

fn clock(seconds: f32) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "\u{2014}".to_owned();
    }
    let total = seconds.round() as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

fn fill_row(renderer: &mut Renderer, bounds: Rectangle, color: Color) {
    fill(renderer, bounds, color, radius());
}

fn fill(renderer: &mut Renderer, bounds: Rectangle, color: Color, corner: f32) {
    use iced::advanced::Renderer as _;

    renderer.fill_quad(
        Quad {
            bounds,
            border: Border {
                radius: corner.into(),
                ..Border::default()
            },
            ..Quad::default()
        },
        Background::Color(color),
    );
}

fn draw_icon(renderer: &mut Renderer, handle: &Handle, cell: Rectangle, size: f32, color: Color) {
    use iced::advanced::svg::Renderer as _;

    let bounds = Rectangle {
        x: cell.x + (cell.width - size) / 2.0,
        y: cell.y + (cell.height - size) / 2.0,
        width: size,
        height: size,
    };

    renderer.draw_svg(
        svg::Svg {
            handle: handle.clone(),
            color: Some(color),
            rotation: iced::Radians(0.0),
            opacity: 1.0,
        },
        bounds,
        cell,
    );
}

fn draw_text(
    renderer: &mut Renderer,
    content: &str,
    bounds: Rectangle,
    size: f32,
    color: Color,
    align_x: text::Alignment,
) {
    let x = match align_x {
        text::Alignment::Center => bounds.center_x(),
        text::Alignment::Right => bounds.x + bounds.width,
        _ => bounds.x,
    };

    renderer.fill_text(
        text::Text {
            content: content.to_owned(),
            bounds: bounds.size(),
            size: size.into(),
            line_height: text::LineHeight::default(),
            font: renderer.default_font(),
            align_x,
            align_y: Vertical::Center,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::None,
        },
        Point::new(x, bounds.center_y()),
        color,
        bounds,
    );
}

impl<'a, Message: 'a> From<QueueList<'a, Message>> for Element<'a, Message, Theme, Renderer> {
    fn from(list: QueueList<'a, Message>) -> Self {
        Self::new(list)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn metrics(rows: usize) -> RowMetrics {
        RowMetrics {
            bounds: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 300.0,
                height: rows as f32 * ROW_HEIGHT,
            },
            len: rows,
        }
    }

    fn at(x: f32, y: f32) -> mouse::Cursor {
        mouse::Cursor::Available(Point::new(x, y))
    }

    #[test]
    fn each_row_claims_its_own_band() {
        let m = metrics(4);
        for index in 0..4 {
            let y = index as f32 * ROW_HEIGHT + ROW_HEIGHT / 2.0;
            assert_eq!(m.row_at_y(y), Some(index), "y {y}");
        }
    }

    #[test]
    fn a_cursor_inside_the_list_finds_its_row() {
        let m = metrics(4);
        let y = ROW_HEIGHT * 2.0 + ROW_HEIGHT / 2.0;

        assert_eq!(m.row_at(at(50.0, y)), Some(2));
    }

    #[test]
    fn a_cursor_outside_the_list_finds_no_row() {
        let m = metrics(4);

        assert_eq!(m.row_at(at(-20.0, ROW_HEIGHT / 2.0)), None);
        assert_eq!(m.row_at(at(50.0, -20.0)), None);
        assert_eq!(m.row_at(mouse::Cursor::Unavailable), None);
    }

    #[test]
    fn a_cursor_past_the_last_row_is_on_no_row() {
        let m = metrics(3);
        assert_eq!(m.row_at_y(3.0 * ROW_HEIGHT + 1.0), None);
    }

    #[test]
    fn a_cursor_above_the_list_is_on_no_row() {
        let m = metrics(3);
        assert_eq!(m.row_at_y(-5.0), None);
    }

    #[test]
    fn an_empty_list_has_no_rows_to_hit() {
        assert_eq!(metrics(0).row_at_y(0.0), None);
    }

    #[test]
    fn the_grip_sits_inside_its_row() {
        let m = metrics(3);
        let grip = m.grip_bounds(1);
        let row = m.row_bounds(1);

        assert!(grip.y >= row.y && grip.y + grip.height <= row.y + row.height);
        assert!(grip.x >= row.x);
        assert_eq!(grip.width, POSITION_WIDTH);
    }

    #[test]
    fn the_remove_control_sits_inside_its_row() {
        let m = metrics(3);
        let remove = m.remove_bounds(2);
        let row = m.row_bounds(2);

        assert!(remove.y >= row.y, "remove escaped the top of its row");
        assert!(
            remove.y + remove.height <= row.y + row.height + 0.001,
            "remove escaped the bottom of its row"
        );
        assert!(
            remove.x + remove.width <= row.x + row.width,
            "remove escaped the trailing edge"
        );
    }

    #[test]
    fn the_grip_and_the_remove_control_do_not_overlap() {
        let m = metrics(2);
        let grip = m.grip_bounds(0);
        let remove = m.remove_bounds(0);

        assert!(
            grip.x + grip.width <= remove.x,
            "the two hit regions overlap, so one would swallow the other"
        );
    }

    #[test]
    fn the_upper_half_of_a_row_aims_at_the_gap_above_it() {
        let m = metrics(4);
        assert_eq!(m.gap_at_y(ROW_HEIGHT * 2.0 + 2.0), 2);
    }

    #[test]
    fn the_lower_half_of_a_row_aims_at_the_gap_below_it() {
        let m = metrics(4);
        assert_eq!(m.gap_at_y(ROW_HEIGHT * 2.0 + ROW_HEIGHT - 2.0), 3);
    }

    #[test]
    fn dragging_below_the_list_holds_the_last_gap() {
        let m = metrics(4);
        for y in [4.0 * ROW_HEIGHT + 1.0, 4.0 * ROW_HEIGHT + 500.0] {
            assert_eq!(
                m.gap_at_y(y),
                4,
                "a cursor past the end should stay on the final gap"
            );
        }
    }

    #[test]
    fn dragging_above_the_list_holds_the_first_gap() {
        let m = metrics(4);
        assert_eq!(m.gap_at_y(-200.0), 0);
    }

    #[test]
    fn every_gap_is_reachable() {
        let m = metrics(3);
        let reached: std::collections::BTreeSet<usize> = (0..=(3 * ROW_HEIGHT as usize))
            .map(|y| m.gap_at_y(y as f32))
            .collect();

        assert_eq!(
            reached,
            (0..=3).collect(),
            "some drop position could not be aimed at"
        );
    }

    #[test]
    fn dragging_down_accounts_for_the_lifted_row() {
        assert_eq!(drop_target(0, 2), Some(1));
    }

    #[test]
    fn dragging_up_uses_the_gap_directly() {
        assert_eq!(drop_target(3, 1), Some(1));
    }

    #[test]
    fn dropping_into_either_gap_beside_a_row_does_nothing() {
        assert_eq!(drop_target(2, 2), None);
        assert_eq!(drop_target(2, 3), None);
    }

    #[test]
    fn dropping_past_the_end_lands_last() {
        assert_eq!(drop_target(0, 4), Some(3));
    }

    #[test]
    fn a_drag_to_the_top_lands_first() {
        assert_eq!(drop_target(5, 0), Some(0));
    }

    #[test]
    fn only_the_rows_on_screen_are_drawn() {
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 300.0,
            height: 100.0 * ROW_HEIGHT,
        };
        let viewport = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 300.0,
            height: 5.0 * ROW_HEIGHT,
        };

        let range = visible_range(bounds, &viewport, 100);
        assert!(
            range.len() <= 7,
            "drew {} rows for 5 rows of pane",
            range.len()
        );
    }

    #[test]
    fn a_row_straddling_the_bottom_edge_still_draws() {
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 300.0,
            height: 10.0 * ROW_HEIGHT,
        };
        let viewport = Rectangle {
            x: 0.0,
            y: ROW_HEIGHT * 0.5,
            width: 300.0,
            height: ROW_HEIGHT * 2.0,
        };

        let range = visible_range(bounds, &viewport, 10);
        assert!(range.contains(&2), "{range:?} dropped the straddling row");
    }

    #[test]
    fn a_list_scrolled_past_its_end_draws_nothing_out_of_range() {
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 300.0,
            height: 3.0 * ROW_HEIGHT,
        };
        let viewport = Rectangle {
            x: 0.0,
            y: 50.0 * ROW_HEIGHT,
            width: 300.0,
            height: 5.0 * ROW_HEIGHT,
        };

        let range = visible_range(bounds, &viewport, 3);
        assert!(range.end <= 3, "{range:?} ran past the list");
    }

    #[test]
    fn the_two_text_lines_stay_inside_the_row() {
        let row = Rectangle {
            x: 0.0,
            y: 100.0,
            width: 300.0,
            height: ROW_HEIGHT,
        };
        let (title, artist) = two_lines(row);

        assert!(title >= row.y, "the title escaped the top of the row");
        assert!(
            artist + LABEL_SIZE <= row.y + row.height,
            "the artist escaped the bottom of the row"
        );
        assert!(artist > title, "the artist should sit below the title");
    }

    #[test]
    fn the_text_lines_are_centred_as_a_pair() {
        let row = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 300.0,
            height: ROW_HEIGHT,
        };
        let (title, artist) = two_lines(row);

        let above = title - row.y;
        let below = (row.y + row.height) - (artist + LABEL_SIZE);
        assert!(
            (above - below).abs() < 0.001,
            "the pair sits {above} from the top and {below} from the bottom"
        );
    }

    #[test]
    fn durations_format_as_minutes_and_seconds() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(9.0), "0:09");
        assert_eq!(clock(449.0), "7:29");
    }

    #[test]
    fn an_unknown_duration_is_not_drawn_as_a_time() {
        assert_eq!(clock(-1.0), "\u{2014}");
        assert_eq!(clock(f32::NAN), "\u{2014}");
    }
}
