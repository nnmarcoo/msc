//! Renders one pane.
//!
//! Normal mode draws only content. Edit mode overlays controls on top of the
//! same content via `stack!`, so the content is never displaced and the user
//! edits a true preview.
//!
//! While a pane is being dragged, every pane becomes a hover target: it wraps
//! its content in `responsive` so the closure knows the pane's own size, then a
//! `mouse_area` reports the cursor's fractional position as a [`DropZone`]. The
//! target pane paints a highlight over the zone that a drop would land in.

use iced::alignment::{Horizontal, Vertical};
use iced::widget::svg::Handle;
use iced::widget::tooltip::Position;
use iced::widget::{
    button, column, container, mouse_area, responsive, row, stack, svg, text, tooltip,
};
use iced::{Element, Length};

use crate::app::{DropTarget, Message};
use crate::layout::{Axis, DropZone, PaneId};
use crate::pane::PaneKind;
use crate::styles::{self, LABEL_FONT_SIZE, PAD, TOOLTIP_DELAY};

const ROOT_BAND: f32 = 24.0;

/// Edit-mode control size, matching `LABEL_FONT_SIZE` so icons sit level with
/// any text controls in the same bar.
const ICON_SIZE: f32 = 14.0;

const ICON_SPLIT_VERTICAL: &[u8] = include_bytes!("../../../assets/icons/split_vertical.svg");
const ICON_SPLIT_HORIZONTAL: &[u8] = include_bytes!("../../../assets/icons/split_horizontal.svg");
const ICON_CLOSE: &[u8] = include_bytes!("../../../assets/icons/close.svg");
const ICON_GRIP: &[u8] = include_bytes!("../../../assets/icons/grip.svg");
const ICON_LOCK: &[u8] = include_bytes!("../../../assets/icons/lock.svg");
const ICON_UNLOCK: &[u8] = include_bytes!("../../../assets/icons/unlock.svg");
const ICON_CYCLE: &[u8] = include_bytes!("../../../assets/icons/cycle.svg");

#[derive(Clone, Copy)]
pub struct DragContext {
    pub active: bool,
    pub drop_zone: Option<DropZone>,
}

pub fn view<'a>(
    id: PaneId,
    kind: PaneKind,
    locked: bool,
    edit_mode: bool,
    drag: DragContext,
) -> Element<'a, Message> {
    if !edit_mode {
        return content(kind);
    }

    let mut layers = stack![content(kind)];

    if let Some(zone) = drag.drop_zone {
        layers = layers.push(zone_highlight(zone));
    }

    if drag.active {
        layers = layers.push(hover_sensor(id));
    } else {
        layers = layers.push(edit_overlay(id, kind, locked));
    }

    layers.into()
}

fn content<'a>(kind: PaneKind) -> Element<'a, Message> {
    container(text(kind.title()).size(18))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

pub fn root_edge_band(
    body: Element<'_, Message>,
    active: Option<DropZone>,
) -> Element<'_, Message> {
    let mut layers = stack![body];

    if let Some(edge) = active {
        layers = layers.push(zone_highlight(edge));
    }

    layers = layers
        .push(edge_sensor(DropZone::Left))
        .push(edge_sensor(DropZone::Right))
        .push(edge_sensor(DropZone::Top))
        .push(edge_sensor(DropZone::Bottom));

    layers.into()
}

fn edge_sensor<'a>(edge: DropZone) -> Element<'a, Message> {
    let (align_x, align_y, width, height) = match edge {
        DropZone::Left => (
            Horizontal::Left,
            Vertical::Center,
            Length::Fixed(ROOT_BAND),
            Length::Fill,
        ),
        DropZone::Right => (
            Horizontal::Right,
            Vertical::Center,
            Length::Fixed(ROOT_BAND),
            Length::Fill,
        ),
        DropZone::Top => (
            Horizontal::Center,
            Vertical::Top,
            Length::Fill,
            Length::Fixed(ROOT_BAND),
        ),
        DropZone::Bottom => (
            Horizontal::Center,
            Vertical::Bottom,
            Length::Fill,
            Length::Fixed(ROOT_BAND),
        ),
        DropZone::Center => (
            Horizontal::Center,
            Vertical::Center,
            Length::Fill,
            Length::Fill,
        ),
    };

    let sensor = mouse_area(
        container(iced::widget::Space::new())
            .width(width)
            .height(height),
    )
    .on_enter(Message::DropHovered(DropTarget::RootEdge(edge)))
    .on_exit(Message::DropHoverEnded(DropTarget::RootEdge(edge)));

    container(sensor)
        .align_x(align_x)
        .align_y(align_y)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn hover_sensor<'a>(id: PaneId) -> Element<'a, Message> {
    responsive(move |size| {
        mouse_area(
            container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_move(move |point| {
            let zone = DropZone::from_fraction(point.x / size.width, point.y / size.height);
            Message::DropHovered(DropTarget::Pane(id, zone))
        })
        .on_exit(Message::DropHoverEnded(DropTarget::Pane(
            id,
            DropZone::Center,
        )))
        .into()
    })
    .into()
}

fn zone_highlight<'a>(zone: DropZone) -> Element<'a, Message> {
    let lit = || {
        container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(styles::drop_highlight_style)
    };
    let gap = || {
        container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
    };

    let split: Element<'a, Message> = match zone {
        DropZone::Left => row![lit(), gap()].into(),
        DropZone::Right => row![gap(), lit()].into(),
        DropZone::Top => column![lit(), gap()].into(),
        DropZone::Bottom => column![gap(), lit()].into(),
        DropZone::Center => lit().into(),
    };

    container(split)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn edit_overlay<'a>(id: PaneId, kind: PaneKind, locked: bool) -> Element<'a, Message> {
    responsive(move |pane_size| edit_controls(id, kind, locked, pane_size)).into()
}

fn edit_controls<'a>(
    id: PaneId,
    kind: PaneKind,
    locked: bool,
    pane_size: iced::Size,
) -> Element<'a, Message> {
    let controls = row![
        grab_handle(id),
        svg_button(
            if locked { ICON_LOCK } else { ICON_UNLOCK },
            if locked { "Unlock pane" } else { "Lock pane" },
            Message::ToggleLock(id, pane_size),
        ),
        svg_button(
            ICON_SPLIT_VERTICAL,
            "Split vertically",
            Message::SplitPane(id, Axis::Vertical),
        ),
        svg_button(
            ICON_SPLIT_HORIZONTAL,
            "Split horizontally",
            Message::SplitPane(id, Axis::Horizontal),
        ),
        kind_cycle(id, kind),
        svg_button(ICON_CLOSE, "Close pane", Message::ClosePane(id)),
    ]
    .spacing(PAD / 2.0);

    container(
        container(controls)
            .padding(PAD / 2.0)
            .style(styles::bar_style),
    )
    .align_x(Horizontal::Right)
    .align_y(Vertical::Top)
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(PAD)
    .into()
}

fn grab_handle<'a>(id: PaneId) -> Element<'a, Message> {
    let handle = mouse_area(
        container(icon(ICON_GRIP))
            .padding([2.0, PAD])
            .style(styles::icon_button_style_container),
    )
    .interaction(iced::mouse::Interaction::Grab)
    .on_press(Message::PaneGrabbed(id));
    with_tooltip(handle, "Drag to move")
}

fn kind_cycle<'a>(id: PaneId, kind: PaneKind) -> Element<'a, Message> {
    let next = next_kind(kind);
    let control = button(
        row![icon(ICON_CYCLE), text(kind.title()).size(LABEL_FONT_SIZE)]
            .spacing(PAD / 2.0)
            .align_y(Vertical::Center),
    )
    .on_press(Message::SetPaneKind(id, next))
    .padding([2.0, PAD])
    .style(styles::icon_button_style);
    with_tooltip(control, "Change pane type")
}

/// A bare theme-tinted icon at the standard edit-control size.
fn icon<'a>(bytes: &'static [u8]) -> Element<'a, Message> {
    svg(Handle::from_memory(bytes))
        .style(styles::svg_style)
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .into()
}

/// An icon wrapped in a hoverable, tooltipped button — the edit bar's standard control.
fn svg_button<'a>(bytes: &'static [u8], label: &'a str, message: Message) -> Element<'a, Message> {
    let control = button(icon(bytes))
        .on_press(message)
        .padding([2.0, PAD])
        .style(styles::icon_button_style);
    with_tooltip(control, label)
}

/// Attaches a delayed tooltip above `content`, styled as a floating panel.
fn with_tooltip<'a>(
    content: impl Into<Element<'a, Message>>,
    label: &'a str,
) -> Element<'a, Message> {
    tooltip(
        content,
        container(text(label).size(LABEL_FONT_SIZE))
            .padding(PAD)
            .style(styles::tooltip_style),
        Position::Bottom,
    )
    .delay(TOOLTIP_DELAY)
    .gap(PAD)
    .into()
}

fn next_kind(kind: PaneKind) -> PaneKind {
    let all = PaneKind::ALL;
    let index = all.iter().position(|&k| k == kind).unwrap_or(0);
    all[(index + 1) % all.len()]
}
