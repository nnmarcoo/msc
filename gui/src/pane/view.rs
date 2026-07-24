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
use iced::widget::{button, column, container, mouse_area, responsive, row, stack, text};
use iced::{Element, Length};

use crate::app::{DropTarget, Message};
use crate::layout::{Axis, DropZone, PaneId};
use crate::pane::PaneKind;
use crate::styles::{self, LABEL_FONT_SIZE, PAD};

const ROOT_BAND: f32 = 24.0;

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
    let controls = row![
        grab_handle(id),
        pill(if locked { "🔒" } else { "🔓" }, Message::ToggleLock(id)),
        pill("↔", Message::SplitPane(id, Axis::Vertical)),
        pill("↕", Message::SplitPane(id, Axis::Horizontal)),
        kind_cycle(id, kind),
        pill("×", Message::ClosePane(id)),
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
    mouse_area(
        container(text("⠿").size(LABEL_FONT_SIZE))
            .padding([2.0, PAD])
            .style(styles::icon_button_style_container),
    )
    .interaction(iced::mouse::Interaction::Grab)
    .on_press(Message::PaneGrabbed(id))
    .into()
}

fn kind_cycle<'a>(id: PaneId, kind: PaneKind) -> Element<'a, Message> {
    let next = next_kind(kind);
    button(text(kind.title()).size(LABEL_FONT_SIZE))
        .on_press(Message::SetPaneKind(id, next))
        .padding([2.0, PAD])
        .style(styles::icon_button_style)
        .into()
}

fn pill(label: &str, message: Message) -> Element<'_, Message> {
    button(text(label).size(LABEL_FONT_SIZE))
        .on_press(message)
        .padding([2.0, PAD])
        .style(styles::icon_button_style)
        .into()
}

fn next_kind(kind: PaneKind) -> PaneKind {
    let all = PaneKind::ALL;
    let index = all.iter().position(|&k| k == kind).unwrap_or(0);
    all[(index + 1) % all.len()]
}
