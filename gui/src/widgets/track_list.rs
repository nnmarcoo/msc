//! A virtualized list of tracks.
//!
//! This is a custom widget because a row is not worth an `Element`. Composing a
//! `column` of containers builds, and diffs, one widget tree per track every
//! frame, which a library of any size cannot afford. Here a row is drawn
//! directly with `fill_quad` and `fill_text`, and [`Widget::layout`] reports the
//! full height from a multiplication rather than by measuring children, so the
//! cost of a frame follows the *window*, not the library.
//!
//! Virtualization falls out of the `viewport` iced already passes to `draw`.
//! Inside a `scrollable` that rectangle is the visible slice in content
//! coordinates, so the first and last rows worth drawing are two divisions, and
//! everything above or below is skipped. Scrolling itself stays with the
//! `scrollable`; this widget never tracks an offset, which is what keeps the two
//! from disagreeing about where the list is.
//!
//! Rows draw in three independent layers, one each for playing, selected and
//! hovered, because
//! a track can be all three at once and a single "state" would have to rank
//! them. The highlight spans the full row width rather than the text, so the
//! whole strip is one target.
//!
//! Hover is published only when the row under the cursor *changes*. A cursor
//! moving within one row emits nothing, since every message redraws the window
//! and hover fires on every mouse motion.
//!
//! Selection lives in [`crate::browsing::Selection`] on the app, not here:
//! it is keyed on track ids and read by other panes, so the widget reports what
//! the user did ([`Op`]) and never decides what is selected. [`RowClick`] names
//! what a click's modifiers meant, so the app never re-reads raw key state. Mouse
//! events carry no modifiers of their own, so `State::modifiers` tracks them
//! from the keyboard events that do report them and reads them back on a press.
//!
//! A right-press is deliberately *not* captured: the context menu wrapping this
//! widget needs the same press to know where to open. `Op::RightClicked` only
//! tells the app which row it landed on.
//!
//! `visible_range` is the whole of the virtualization, kept a free function so
//! it can be tested without constructing tracks. It includes one row beyond the
//! viewport so a row straddling the bottom edge still draws. A list shorter than
//! its pane still claims the pane's full height, or the pane's background shows
//! through beneath the last row and the list looks half-drawn.
//!
//! [`Column::value`] borrows from the track wherever it can, so drawing a cell
//! copies no text the track already owns; only `Duration`, which is formatted,
//! allocates. `fill_text` wants an owned `String` regardless, so one allocation
//! per visible cell survives, bounded by the viewport rather than the library,
//! which is the property that matters here.
//!
//! Columns are shares of the row, except `Duration`, which is fixed: its content
//! has a bounded width and a proportional share would leave it swimming in a
//! wide pane. [`header`] is a separate widget so it can stay put while the list
//! scrolls beneath it, and it is deliberately *not* wrapped in `responsive`:
//! that reports `Length::Fill` on both axes, so in a column beside the list it
//! would claim half the pane. It takes its width from its own layout bounds.
//!
//! Enter activates the row under the cursor, since the list has no keyboard
//! focus ring of its own yet and hover is what "here" means.

use std::borrow::Cow;

use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::Renderer as _;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, text};
use iced::alignment::Vertical;
use iced::keyboard::key::Named;
use iced::keyboard::{self, Key};
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme,
    mouse,
};

use verse_core::Track;

use crate::app::RowClick;
use crate::browsing::Context;
use crate::styles::radius;

pub const ROW_HEIGHT: f32 = 26.0;
pub const HEADER_HEIGHT: f32 = 24.0;

const TEXT_SIZE: f32 = 12.0;
const HEADER_TEXT_SIZE: f32 = 11.0;
const PADDING_H: f32 = 10.0;
const COLUMN_GAP: f32 = 10.0;

const DOUBLE_CLICK: std::time::Duration = std::time::Duration::from_millis(400);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Clicked(usize, RowClick),
    Activated(usize),
    RightClicked(usize),
    Hovered(Option<i64>),
    SelectAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Title,
    Artist,
    Album,
    Duration,
}

impl Column {
    pub const ALL: [Column; 4] = [
        Column::Title,
        Column::Artist,
        Column::Album,
        Column::Duration,
    ];

    fn title(self) -> &'static str {
        match self {
            Column::Title => "Title",
            Column::Artist => "Artist",
            Column::Album => "Album",
            Column::Duration => "Duration",
        }
    }

    fn portion(self) -> f32 {
        match self {
            Column::Title => 3.0,
            Column::Artist | Column::Album => 2.0,
            Column::Duration => 0.0,
        }
    }

    fn fixed(self) -> Option<f32> {
        match self {
            Column::Duration => Some(70.0),
            _ => None,
        }
    }

    fn value(self, track: &Track) -> Cow<'_, str> {
        match self {
            Column::Title => Cow::Borrowed(track.title().unwrap_or("\u{2014}")),
            Column::Artist => Cow::Borrowed(track.track_artist().unwrap_or("\u{2014}")),
            Column::Album => Cow::Borrowed(track.album().unwrap_or("\u{2014}")),
            Column::Duration => Cow::Owned(format_duration(track.duration())),
        }
    }
}

fn format_duration(seconds: f32) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "\u{2014}".to_owned();
    }
    let total = seconds.round() as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

fn columns(width: f32) -> Vec<(Column, f32, f32)> {
    let gaps = COLUMN_GAP * (Column::ALL.len() - 1) as f32;
    let fixed: f32 = Column::ALL.iter().filter_map(|c| c.fixed()).sum();
    let flexible = (width - PADDING_H * 2.0 - gaps - fixed).max(0.0);
    let portions: f32 = Column::ALL.iter().map(|c| c.portion()).sum();

    let mut placed = Vec::with_capacity(Column::ALL.len());
    let mut x = PADDING_H;
    for column in Column::ALL {
        let column_width = column
            .fixed()
            .unwrap_or_else(|| flexible * column.portion() / portions.max(1.0));
        placed.push((column, x, column_width));
        x += column_width + COLUMN_GAP;
    }
    placed
}

#[derive(Default)]
struct State {
    hovered_row: Option<usize>,
    last_press: Option<(usize, std::time::Instant)>,
    modifiers: keyboard::Modifiers,
}

pub struct TrackList<'a, Message> {
    rows: Vec<&'a Track>,
    context: Context<'a>,
    on_op: Box<dyn Fn(Op) -> Message + 'a>,
}

impl<'a, Message> TrackList<'a, Message> {
    pub fn new(
        rows: Vec<&'a Track>,
        context: Context<'a>,
        on_op: impl Fn(Op) -> Message + 'a,
    ) -> Self {
        Self {
            rows,
            context,
            on_op: Box::new(on_op),
        }
    }

    fn row_at(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        let position = cursor.position_over(bounds)?;
        let index = ((position.y - bounds.y) / ROW_HEIGHT).floor();
        if index < 0.0 {
            return None;
        }
        let index = index as usize;
        (index < self.rows.len()).then_some(index)
    }

    fn visible_range(&self, bounds: Rectangle, viewport: &Rectangle) -> std::ops::Range<usize> {
        visible_range(bounds, viewport, self.rows.len())
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

impl<Message> Widget<Message, Theme, Renderer> for TrackList<'_, Message> {
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
        let bounds = layout.bounds();
        let state = tree.state.downcast_mut::<State>();

        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::WheelScrolled { .. }) => {
                let row = self.row_at(bounds, cursor);
                if row != state.hovered_row {
                    state.hovered_row = row;
                    let id = row
                        .and_then(|index| self.rows.get(index))
                        .and_then(|t| t.id());
                    shell.publish((self.on_op)(Op::Hovered(id)));
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorLeft) if state.hovered_row.take().is_some() => {
                shell.publish((self.on_op)(Op::Hovered(None)));
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(index) = self.row_at(bounds, cursor) else {
                    return;
                };

                let now = std::time::Instant::now();
                let repeat = state
                    .last_press
                    .is_some_and(|(last, at)| last == index && now - at < DOUBLE_CLICK);

                if repeat {
                    state.last_press = None;
                    shell.publish((self.on_op)(Op::Activated(index)));
                } else {
                    state.last_press = Some((index, now));
                    let click = RowClick::from_modifiers(state.modifiers);
                    shell.publish((self.on_op)(Op::Clicked(index, click)));
                }

                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if let Some(index) = self.row_at(bounds, cursor) {
                    shell.publish((self.on_op)(Op::RightClicked(index)));
                    shell.request_redraw();
                }
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                state.modifiers = *modifiers;

                if modifiers.command() && matches!(key, Key::Character(c) if c.as_str() == "a") {
                    shell.publish((self.on_op)(Op::SelectAll));
                    shell.capture_event();
                    shell.request_redraw();
                }
                if matches!(key, Key::Named(Named::Enter))
                    && let Some(index) = state.hovered_row
                {
                    shell.publish((self.on_op)(Op::Activated(index)));
                    shell.capture_event();
                }
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;

        let bounds = layout.bounds();
        let palette = theme.extended_palette();
        let placed = columns(bounds.width);

        for index in self.visible_range(bounds, viewport) {
            let Some(track) = self.rows.get(index) else {
                continue;
            };
            let row = Rectangle {
                x: bounds.x,
                y: bounds.y + index as f32 * ROW_HEIGHT,
                width: bounds.width,
                height: ROW_HEIGHT,
            };

            let state = track
                .id()
                .map(|id| self.context.row_state(id))
                .unwrap_or_default();

            if state.selected {
                fill_row(renderer, row, palette.primary.base.color.scale_alpha(0.30));
            }
            if state.hovered {
                fill_row(
                    renderer,
                    row,
                    palette.background.strong.color.scale_alpha(0.55),
                );
            }
            if state.playing {
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle { width: 2.0, ..row },
                        ..Quad::default()
                    },
                    Background::Color(palette.primary.base.color),
                );
            }

            let color = if state.playing {
                palette.primary.base.color
            } else {
                palette.background.base.text
            };

            for (column, x, width) in &placed {
                if *width <= 0.0 {
                    continue;
                }
                draw_cell(
                    renderer,
                    &column.value(track),
                    Rectangle {
                        x: row.x + x,
                        y: row.y,
                        width: *width,
                        height: ROW_HEIGHT,
                    },
                    TEXT_SIZE,
                    color,
                );
            }
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.row_at(layout.bounds(), cursor).is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

pub fn header<'a, Message: 'a>() -> Element<'a, Message, Theme, Renderer> {
    Header.into()
}

struct Header;

impl<Message> Widget<Message, Theme, Renderer> for Header {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fixed(HEADER_HEIGHT),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, Length::Fill, Length::Fixed(HEADER_HEIGHT))
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let palette = theme.extended_palette();

        for (column, x, width) in columns(bounds.width) {
            if width <= 0.0 {
                continue;
            }
            draw_cell(
                renderer,
                column.title(),
                Rectangle {
                    x: bounds.x + x,
                    y: bounds.y,
                    width,
                    height: bounds.height,
                },
                HEADER_TEXT_SIZE,
                palette.background.base.text.scale_alpha(0.6),
            );
        }
    }
}

impl<'a, Message: 'a> From<Header> for Element<'a, Message, Theme, Renderer> {
    fn from(header: Header) -> Self {
        Self::new(header)
    }
}

fn fill_row(renderer: &mut Renderer, bounds: Rectangle, color: Color) {
    use iced::advanced::Renderer as _;

    renderer.fill_quad(
        Quad {
            bounds,
            border: Border {
                radius: radius().into(),
                ..Border::default()
            },
            ..Quad::default()
        },
        Background::Color(color),
    );
}

fn draw_cell(renderer: &mut Renderer, content: &str, bounds: Rectangle, size: f32, color: Color) {
    renderer.fill_text(
        text::Text {
            content: content.to_owned(),
            bounds: bounds.size(),
            size: size.into(),
            line_height: text::LineHeight::default(),
            font: renderer.default_font(),
            align_x: text::Alignment::Left,
            align_y: Vertical::Center,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::None,
        },
        Point::new(bounds.x, bounds.center_y()),
        color,
        bounds,
    );
}

impl<'a, Message: 'a> From<TrackList<'a, Message>> for Element<'a, Message, Theme, Renderer> {
    fn from(list: TrackList<'a, Message>) -> Self {
        Self::new(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list_bounds(len: usize) -> Rectangle {
        Rectangle {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: len as f32 * ROW_HEIGHT,
        }
    }

    fn window(y: f32, height: f32) -> Rectangle {
        Rectangle {
            x: 0.0,
            y,
            width: 600.0,
            height,
        }
    }

    #[test]
    fn only_the_rows_on_screen_are_drawn() {
        let len = 50_000;
        let range = visible_range(list_bounds(len), &window(0.0, 400.0), len);
        assert!(
            range.len() <= (400.0 / ROW_HEIGHT).ceil() as usize + 1,
            "a 400px window drew {} of {len} rows",
            range.len()
        );
    }

    #[test]
    fn the_cost_of_a_frame_does_not_grow_with_the_library() {
        let viewport = window(0.0, 400.0);
        let small = visible_range(list_bounds(100), &viewport, 100).len();
        let huge = visible_range(list_bounds(100_000), &viewport, 100_000).len();
        assert_eq!(small, huge, "a larger library drew more rows");
    }

    #[test]
    fn scrolling_moves_the_window_down_the_list() {
        let len = 1_000;
        let bounds = list_bounds(len);
        let top = visible_range(bounds, &window(0.0, 300.0), len);
        let scrolled = visible_range(bounds, &window(ROW_HEIGHT * 400.0, 300.0), len);

        assert_eq!(top.start, 0);
        assert_eq!(scrolled.start, 400);
        assert_eq!(top.len(), scrolled.len());
    }

    #[test]
    fn a_row_straddling_the_bottom_edge_still_draws() {
        let len = 100;
        let height = ROW_HEIGHT * 9.5;
        let range = visible_range(list_bounds(len), &window(0.0, height), len);
        assert!(
            range.end >= 10,
            "the partially visible tenth row was skipped: {range:?}"
        );
    }

    #[test]
    fn the_range_never_runs_past_the_last_row() {
        let len = 12;
        let range = visible_range(list_bounds(len), &window(0.0, 5_000.0), len);
        assert_eq!(range.end, len);
    }

    #[test]
    fn scrolling_past_the_end_yields_nothing_to_draw() {
        let len = 10;
        let range = visible_range(list_bounds(len), &window(10_000.0, 400.0), len);
        assert!(range.is_empty(), "{range:?}");
    }

    #[test]
    fn an_empty_list_draws_no_rows() {
        let range = visible_range(list_bounds(0), &window(0.0, 400.0), 0);
        assert!(range.is_empty());
    }

    #[test]
    fn every_row_is_reachable_by_scrolling() {
        let len = 500;
        let bounds = list_bounds(len);
        let height = 200.0;
        let mut seen = vec![false; len];

        let mut y = 0.0;
        while y < bounds.height {
            for index in visible_range(bounds, &window(y, height), len) {
                seen[index] = true;
            }
            y += height;
        }

        assert!(
            seen.iter().all(|&hit| hit),
            "{} rows were never drawn at any scroll offset",
            seen.iter().filter(|hit| !**hit).count()
        );
    }

    #[test]
    fn columns_span_the_row_without_overflowing() {
        let width = 600.0;
        let placed = columns(width);
        let (_, last_x, last_width) = placed.last().copied().expect("four columns");
        assert!(
            last_x + last_width <= width - PADDING_H + 0.01,
            "columns ran to {} in a {width}px row",
            last_x + last_width
        );
    }

    #[test]
    fn duration_keeps_a_fixed_width_as_the_row_grows() {
        let narrow = columns(400.0);
        let wide = columns(1600.0);
        let duration = |placed: &[(Column, f32, f32)]| {
            placed
                .iter()
                .find(|(c, _, _)| *c == Column::Duration)
                .map(|(_, _, w)| *w)
                .expect("a duration column")
        };
        assert!((duration(&narrow) - duration(&wide)).abs() < 0.01);
    }

    #[test]
    fn flexible_columns_grow_with_the_row() {
        let title = |width: f32| {
            columns(width)
                .into_iter()
                .find(|(c, _, _)| *c == Column::Title)
                .map(|(_, _, w)| w)
                .expect("a title column")
        };
        assert!(title(1600.0) > title(400.0));
    }

    #[test]
    fn title_is_the_widest_flexible_column() {
        let placed = columns(900.0);
        let width_of = |wanted: Column| {
            placed
                .iter()
                .find(|(c, _, _)| *c == wanted)
                .map(|(_, _, w)| *w)
                .expect("column")
        };
        assert!(width_of(Column::Title) > width_of(Column::Artist));
        assert!((width_of(Column::Artist) - width_of(Column::Album)).abs() < 0.01);
    }

    #[test]
    fn a_row_that_cannot_fit_gets_no_negative_width() {
        for width in [0.0, 10.0, 60.0, 120.0] {
            for (column, _, placed) in columns(width) {
                assert!(
                    placed >= 0.0,
                    "{column:?} was {placed}px wide in a {width}px row"
                );
            }
        }
    }

    #[test]
    fn durations_format_as_minutes_and_seconds() {
        assert_eq!(format_duration(0.0), "0:00");
        assert_eq!(format_duration(61.0), "1:01");
        assert_eq!(format_duration(599.6), "10:00");
        assert_eq!(format_duration(3600.0), "60:00");
    }

    #[test]
    fn a_nonsense_duration_does_not_render_as_a_time() {
        assert_eq!(format_duration(-1.0), "\u{2014}");
        assert_eq!(format_duration(f32::NAN), "\u{2014}");
    }

    #[test]
    fn every_column_has_a_width_rule() {
        for column in Column::ALL {
            assert!(
                column.fixed().is_some() || column.portion() > 0.0,
                "{column:?} would be invisible: no fixed width and no portion"
            );
        }
    }
}
