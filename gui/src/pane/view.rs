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
//!
//! The edit controls are authored once in visual order and each [`ControlForm`]
//! lays that same sequence out unchanged, so the position a control occupies is
//! learnable across forms: the grab handle sits nearest the pane's top-right
//! anchor and close is always the far end. A narrowing pane loses the picker's
//! text label before it loses the row itself, and only a pane too narrow for
//! five icons in a line falls back to the stack. Width alone decides, since a
//! row that fits is preferable at any height.
//!
//! The stack is taller than the shortest pane the layout allows, so a pane at
//! that floor overflows its controls rather than changing shape a third time.
//!
//! Splitting follows the pane's long axis — a wide pane divides into left and
//! right halves, a tall one into top and bottom, and a square one vertically.
//! That guesses wrong for some intents, so the icon and tooltip both show the
//! axis the current shape would pick and the outcome is visible before the
//! press; a wrong guess costs one click on the new pane's close button, which
//! collapses the split again.

use iced::alignment::{Horizontal, Vertical};
use iced::widget::svg::Handle;
use iced::widget::tooltip::Position;
use iced::widget::{
    button, column, container, mouse_area, responsive, row, stack, svg, text, tooltip,
};
use iced::{Element, Length};

use crate::app::{DropTarget, Message};
use crate::layout::{Axis, DropZone, Locks, PaneId, PaneMetrics};
use crate::pane::{PaneKind, controls};
use crate::styles::{self, LABEL_FONT_SIZE, PAD, TOOLTIP_DELAY};
use crate::widgets::pane_picker::PanePicker;

const ROOT_BAND: f32 = 24.0;

const ICON_SIZE: f32 = 14.0;

const ICON_SPLIT_VERTICAL: &[u8] = include_bytes!("../../../assets/icons/split_vertical.svg");
const ICON_SPLIT_HORIZONTAL: &[u8] = include_bytes!("../../../assets/icons/split_horizontal.svg");
const ICON_CLOSE: &[u8] = include_bytes!("../../../assets/icons/close.svg");
const ICON_GRIP: &[u8] = include_bytes!("../../../assets/icons/grip.svg");
const ICON_LOCK_WIDTH: &[u8] = include_bytes!("../../../assets/icons/lock_width.svg");
const ICON_LOCK_HEIGHT: &[u8] = include_bytes!("../../../assets/icons/lock_height.svg");
const ICON_LOCK_BOTH: &[u8] = include_bytes!("../../../assets/icons/lock_both.svg");
const ICON_UNLOCK: &[u8] = include_bytes!("../../../assets/icons/unlock.svg");

#[derive(Clone, Copy)]
pub struct DragContext {
    pub active: bool,
    pub drop_zone: Option<DropZone>,
}

#[derive(Clone, Copy, Default)]
pub struct Playback {
    pub is_playing: bool,
}

pub fn view<'a>(
    id: PaneId,
    kind: PaneKind,
    locks: Locks,
    edit_mode: bool,
    drag: DragContext,
    playback: Playback,
    span: iced::Size,
) -> Element<'a, Message> {
    if !edit_mode {
        return content(kind, playback);
    }

    let mut layers = stack![content(kind, playback)];

    if let Some(zone) = drag.drop_zone {
        layers = layers.push(zone_highlight(zone));
    }

    if drag.active {
        layers = layers.push(hover_sensor(id));
    } else {
        layers = layers.push(edit_overlay(id, kind, locks, span));
    }

    layers.into()
}

fn content<'a>(kind: PaneKind, playback: Playback) -> Element<'a, Message> {
    match kind {
        PaneKind::Controls => controls::view(playback.is_playing),
        _ => container(text(kind.title()).size(18))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
    }
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

fn edit_overlay<'a>(
    id: PaneId,
    kind: PaneKind,
    locks: Locks,
    span: iced::Size,
) -> Element<'a, Message> {
    responsive(move |pane_size| edit_controls(id, kind, locks, pane_size, span)).into()
}

fn edit_controls<'a>(
    id: PaneId,
    kind: PaneKind,
    locks: Locks,
    pane_size: iced::Size,
    span: iced::Size,
) -> Element<'a, Message> {
    let form = ControlForm::pick(pane_size, kind);

    let buttons = [
        grab_handle(id),
        lock_button(id, locks, pane_size, span),
        kind_picker(id, kind, form.is_compact()),
        split_button(id, pane_size),
        svg_button(ICON_CLOSE, "Close pane", Message::ClosePane(id)),
    ];

    let controls: Element<'a, Message> = match form {
        ControlForm::Labelled | ControlForm::Compact => row(buttons).spacing(GAP).into(),
        ControlForm::Vertical => column(buttons).spacing(GAP).into(),
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
    Labelled,
    Compact,
    Vertical,
}

const BUTTON: f32 = ICON_SIZE + PAD * 2.0;
const GAP: f32 = PAD / 2.0;
const CHROME: f32 = PAD + PAD * 2.0;

const CONTROLS: f32 = 5.0;

impl ControlForm {
    fn pick(pane: iced::Size, kind: PaneKind) -> Self {
        if pane.width >= Self::labelled_width(kind) {
            Self::Labelled
        } else if pane.width >= Self::compact_width() {
            Self::Compact
        } else {
            Self::Vertical
        }
    }

    fn is_compact(self) -> bool {
        self != Self::Labelled
    }

    fn labelled_width(kind: PaneKind) -> f32 {
        Self::row_width(PanePicker::<Message>::label_width(kind))
    }

    fn compact_width() -> f32 {
        Self::row_width(PanePicker::<Message>::compact_width())
    }

    fn row_width(picker: f32) -> f32 {
        (CONTROLS - 1.0) * BUTTON + picker + (CONTROLS - 1.0) * GAP + CHROME
    }

    fn vertical_height() -> f32 {
        CONTROLS * BUTTON + (CONTROLS - 1.0) * GAP + CHROME
    }
}

fn split_axis_for(pane_size: iced::Size) -> Axis {
    if pane_size.height > pane_size.width {
        Axis::Horizontal
    } else {
        Axis::Vertical
    }
}

fn split_icon_for(pane_size: iced::Size) -> &'static [u8] {
    match split_axis_for(pane_size) {
        Axis::Horizontal => ICON_SPLIT_HORIZONTAL,
        Axis::Vertical => ICON_SPLIT_VERTICAL,
    }
}

fn split_button<'a>(id: PaneId, pane_size: iced::Size) -> Element<'a, Message> {
    let axis = split_axis_for(pane_size);
    let label = match axis {
        Axis::Horizontal => "Split into top and bottom",
        Axis::Vertical => "Split into left and right",
    };
    svg_button(
        split_icon_for(pane_size),
        label,
        Message::SplitPane(id, axis),
    )
}

fn lock_button<'a>(
    id: PaneId,
    locks: Locks,
    pane_size: iced::Size,
    span: iced::Size,
) -> Element<'a, Message> {
    let (icon, label) = match (locks.width, locks.height) {
        (None, None) => (ICON_UNLOCK, "Free"),
        (Some(_), None) => (ICON_LOCK_WIDTH, "Width locked"),
        (None, Some(_)) => (ICON_LOCK_HEIGHT, "Height locked"),
        (Some(_), Some(_)) => (ICON_LOCK_BOTH, "Width and height locked"),
    };
    svg_button(
        icon,
        label,
        Message::CycleLock(
            id,
            PaneMetrics {
                pane: pane_size,
                span,
            },
        ),
    )
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
    fn wide_pane_keeps_the_label() {
        let width = ControlForm::labelled_width(KIND);
        assert_eq!(
            ControlForm::pick(Size::new(width, 400.0), KIND),
            ControlForm::Labelled
        );
    }

    #[test]
    fn wide_but_short_pane_still_uses_a_row() {
        let width = ControlForm::labelled_width(KIND);
        assert_eq!(
            ControlForm::pick(Size::new(width, 40.0), KIND),
            ControlForm::Labelled
        );
    }

    #[test]
    fn narrow_pane_compacts_the_picker_before_stacking() {
        let width = ControlForm::labelled_width(KIND) - 1.0;
        assert_eq!(
            ControlForm::pick(Size::new(width, 400.0), KIND),
            ControlForm::Compact
        );
    }

    #[test]
    fn a_compact_row_is_still_a_row_when_the_pane_is_short() {
        let width = ControlForm::compact_width();
        assert_eq!(
            ControlForm::pick(Size::new(width, 40.0), KIND),
            ControlForm::Compact
        );
    }

    #[test]
    fn too_narrow_for_five_icons_falls_back_to_vertical() {
        let width = ControlForm::compact_width() - 1.0;
        assert_eq!(
            ControlForm::pick(Size::new(width, 400.0), KIND),
            ControlForm::Vertical
        );
    }

    #[test]
    fn smallest_allowed_pane_uses_vertical() {
        let min = crate::layout::MIN_PANE;
        assert_eq!(
            ControlForm::pick(Size::new(min, min), KIND),
            ControlForm::Vertical
        );
    }

    #[test]
    fn compacting_is_the_only_form_that_keeps_the_label() {
        assert!(!ControlForm::Labelled.is_compact());
        assert!(ControlForm::Compact.is_compact());
        assert!(ControlForm::Vertical.is_compact());
    }

    #[test]
    fn compact_row_is_narrower_than_any_labelled_one() {
        let compact = ControlForm::compact_width();
        for kind in PaneKind::ALL {
            assert!(
                compact <= ControlForm::labelled_width(kind),
                "{kind:?} labelled row was narrower than the compact one"
            );
        }
    }

    #[test]
    fn longer_label_needs_more_width() {
        assert!(
            ControlForm::labelled_width(PaneKind::NowPlaying)
                > ControlForm::labelled_width(PaneKind::Empty)
        );
    }

    #[test]
    fn split_follows_the_long_axis() {
        assert_eq!(split_axis_for(Size::new(400.0, 100.0)), Axis::Vertical);
        assert_eq!(split_axis_for(Size::new(100.0, 400.0)), Axis::Horizontal);
    }

    #[test]
    fn square_pane_splits_vertically() {
        assert_eq!(split_axis_for(Size::new(200.0, 200.0)), Axis::Vertical);
    }

    #[test]
    fn split_icon_matches_the_axis_it_would_pick() {
        for size in [Size::new(400.0, 100.0), Size::new(100.0, 400.0)] {
            let expected = match split_axis_for(size) {
                Axis::Vertical => ICON_SPLIT_VERTICAL,
                Axis::Horizontal => ICON_SPLIT_HORIZONTAL,
            };
            assert_eq!(
                split_icon_for(size),
                expected,
                "{size:?} showed an icon for the other axis"
            );
        }
    }

    #[test]
    fn the_compact_thresholds_are_kind_independent() {
        let compact = ControlForm::compact_width();
        for kind in PaneKind::ALL {
            assert_eq!(
                ControlForm::pick(Size::new(compact, 400.0), kind),
                ControlForm::Compact,
                "{kind:?} did not take the compact row at its exact width"
            );
            assert_eq!(
                ControlForm::pick(Size::new(compact - 1.0, 400.0), kind),
                ControlForm::Vertical,
                "{kind:?} did not stack just below the compact width"
            );
        }
    }

    #[test]
    fn the_vertical_stack_overflows_only_the_smallest_panes() {
        let needed = ControlForm::vertical_height();
        assert!(needed > crate::layout::MIN_PANE);
        assert!(
            needed <= 2.0 * crate::layout::MIN_PANE,
            "the stack needs {needed}px, more than two minimum panes tall"
        );
    }
}
