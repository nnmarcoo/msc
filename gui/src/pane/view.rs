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
use crate::widgets::pane_picker::PanePicker;

const ROOT_BAND: f32 = 24.0;

const ICON_SIZE: f32 = 14.0;

const ICON_SPLIT_VERTICAL: &[u8] = include_bytes!("../../../assets/icons/split_vertical.svg");
const ICON_SPLIT_HORIZONTAL: &[u8] = include_bytes!("../../../assets/icons/split_horizontal.svg");
const ICON_CLOSE: &[u8] = include_bytes!("../../../assets/icons/close.svg");
const ICON_GRIP: &[u8] = include_bytes!("../../../assets/icons/grip.svg");
const ICON_LOCK: &[u8] = include_bytes!("../../../assets/icons/lock.svg");
const ICON_UNLOCK: &[u8] = include_bytes!("../../../assets/icons/unlock.svg");

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
    let form = ControlForm::pick(pane_size, kind);
    let compact = form != ControlForm::Horizontal;

    let outward = [
        svg_button(ICON_CLOSE, "Close pane", Message::ClosePane(id)),
        kind_picker(id, kind, compact),
        svg_button(
            ICON_SPLIT_HORIZONTAL,
            "Split horizontally",
            Message::SplitPane(id, Axis::Horizontal),
        ),
        svg_button(
            ICON_SPLIT_VERTICAL,
            "Split vertically",
            Message::SplitPane(id, Axis::Vertical),
        ),
        svg_button(
            if locked { ICON_LOCK } else { ICON_UNLOCK },
            if locked { "Unlock pane" } else { "Lock pane" },
            Message::ToggleLock(id, pane_size),
        ),
        grab_handle(id),
    ];

    let controls: Element<'a, Message> = match form {
        ControlForm::Horizontal => row(reversed(outward)).spacing(PAD / 2.0).into(),
        ControlForm::Vertical => column(outward).spacing(PAD / 2.0).into(),
        ControlForm::Grid => grid_3x2(outward),
    };

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlForm {
    Horizontal,
    Vertical,
    Grid,
}

const BUTTON: f32 = ICON_SIZE + PAD * 2.0;
const GAP: f32 = PAD / 2.0;
const CHROME: f32 = PAD + PAD * 2.0;

impl ControlForm {
    fn pick(pane: iced::Size, kind: PaneKind) -> Self {
        if pane.width >= Self::horizontal_width(kind) {
            Self::Horizontal
        } else if pane.height >= Self::vertical_height() {
            Self::Vertical
        } else {
            Self::Grid
        }
    }

    fn horizontal_width(kind: PaneKind) -> f32 {
        5.0 * BUTTON + PanePicker::<Message>::label_width(kind) + 5.0 * GAP + CHROME
    }

    fn vertical_height() -> f32 {
        6.0 * BUTTON + 5.0 * GAP + CHROME
    }
}

fn reversed(buttons: [Element<'_, Message>; 6]) -> [Element<'_, Message>; 6] {
    let mut buttons = buttons;
    buttons.reverse();
    buttons
}

fn grid_3x2<'a>(buttons: [Element<'a, Message>; 6]) -> Element<'a, Message> {
    let mut rows = column![].spacing(GAP);
    let mut pending: Vec<Element<'a, Message>> = Vec::with_capacity(3);

    for button in buttons {
        pending.push(button);
        if pending.len() == 3 {
            let mut line = std::mem::replace(&mut pending, Vec::with_capacity(3));
            line.reverse();
            rows = rows.push(row(line).spacing(GAP));
        }
    }

    rows.into()
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

fn kind_picker<'a>(id: PaneId, kind: PaneKind, compact: bool) -> Element<'a, Message> {
    let picker =
        PanePicker::new(kind, move |picked| Message::SetPaneKind(id, picked)).compact(compact);
    let label = if compact {
        kind.title()
    } else {
        "Change pane type"
    };
    with_tooltip(picker, label)
}

fn icon<'a>(bytes: &'static [u8]) -> Element<'a, Message> {
    svg(Handle::from_memory(bytes))
        .style(styles::svg_style)
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .into()
}

fn svg_button<'a>(bytes: &'static [u8], label: &'a str, message: Message) -> Element<'a, Message> {
    let control = button(icon(bytes))
        .on_press(message)
        .padding([2.0, PAD])
        .style(styles::icon_button_style);
    with_tooltip(control, label)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Size;

    const KIND: PaneKind = PaneKind::Library;

    #[test]
    fn wide_pane_uses_horizontal() {
        let width = ControlForm::horizontal_width(KIND);
        assert_eq!(
            ControlForm::pick(Size::new(width, 400.0), KIND),
            ControlForm::Horizontal
        );
    }

    #[test]
    fn wide_but_short_pane_still_uses_horizontal() {
        let width = ControlForm::horizontal_width(KIND);
        assert_eq!(
            ControlForm::pick(Size::new(width, 40.0), KIND),
            ControlForm::Horizontal
        );
    }

    #[test]
    fn narrow_but_tall_pane_uses_vertical() {
        let width = ControlForm::horizontal_width(KIND) - 1.0;
        let height = ControlForm::vertical_height();
        assert_eq!(
            ControlForm::pick(Size::new(width, height), KIND),
            ControlForm::Vertical
        );
    }

    #[test]
    fn narrow_and_short_pane_uses_grid() {
        let width = ControlForm::horizontal_width(KIND) - 1.0;
        let height = ControlForm::vertical_height() - 1.0;
        assert_eq!(
            ControlForm::pick(Size::new(width, height), KIND),
            ControlForm::Grid
        );
    }

    #[test]
    fn smallest_allowed_pane_uses_grid() {
        let min = crate::layout::MIN_PANE;
        assert_eq!(
            ControlForm::pick(Size::new(min, min), KIND),
            ControlForm::Grid
        );
    }

    #[test]
    fn longer_label_needs_more_width() {
        assert!(
            ControlForm::horizontal_width(PaneKind::NowPlaying)
                > ControlForm::horizontal_width(PaneKind::Empty)
        );
    }

    #[test]
    fn vertical_height_is_kind_independent() {
        let height = ControlForm::vertical_height();
        for kind in PaneKind::ALL {
            let width = ControlForm::horizontal_width(kind) - 1.0;
            assert_eq!(
                ControlForm::pick(Size::new(width, height), kind),
                ControlForm::Vertical,
                "{kind:?} did not take the stack at its exact height"
            );
        }
    }
}
