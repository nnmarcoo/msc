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
//! anchor and close is always the far end. A pane too narrow for four icons in a
//! line falls back to the stack. Width alone decides, since a row that fits is
//! preferable at any height.
//!
//! The row holds only what is done repeatedly while arranging a layout —
//! grabbing, splitting, closing. A pane's *settings*, its kind and its size
//! lock, live behind the gear in [`crate::pane::options`]. Splitting
//! deliberately stayed out here rather than joining them: it is how the tree
//! gets built at all, and it would also invalidate the pane whose modal was
//! open. Moving the kind picker out is what let [`ControlForm`] drop from three
//! forms to two — the picker was the only control whose width depended on the
//! kind, which is why a "labeled" form had to exist to measure it, and the row
//! is four identical icon buttons now.
//!
//! The gear carries the pane's measured size in its message, because the
//! `responsive` wrapper around these controls is the only place that knows it
//! and the modal, which covers the pane, cannot ask for it afterwards.
//!
//! The stack is taller than the shortest pane the layout allows, so a pane at
//! that floor overflows its controls rather than changing shape a third time.
//!
//! Two structs split what a pane draws from by how far it varies. [`Shared`]
//! holds what is the same for every pane in the frame, the playback flags and
//! [`Context`], while [`Pane`] holds what identifies this one: its id, kind, its
//! own [`PaneState`], and its settings. Both exist so that giving panes access
//! to something new does not lengthen every signature between the app and the
//! pane. The settings are borrowed rather than owned so [`Pane`] stays `Copy`;
//! they live in the layout for exactly as long as the frame does.
//!
//! [`Playback::bins`] is an owned array and so pushes [`Shared`] past the size
//! clippy wants passed by reference. It stays by value because the analyzer
//! publishes through a triple buffer: reading it yields an owned
//! [`verse_core::VisData`] with nothing behind it to borrow from, and `view`
//! returns an `Element` tied to `&self`, so a borrowed field would have to point
//! at a local that the returned tree outlives. Copying the bins once per frame
//! is what that costs, against a widget tree the same frame rebuilds entirely.
//!
//! `Shared::visible` is the filtered rows, computed once in [`crate::app`] and
//! lent to every pane that lists tracks. Each pane calling `Context::visible`
//! for itself re-ran the search once per pane per frame, which on a large
//! library cost more than the frame budget on its own. It is a slice rather than
//! a `Vec` because [`Shared`] is `Copy` and handed to every pane; the rows it
//! points at live in the app's `view` for exactly as long as the frame does.
//!
//! Splitting follows the pane's long axis, so a wide pane divides into left and
//! right halves, a tall one into top and bottom, and a square one vertically.
//! That guesses wrong for some intents, so the icon and tooltip both show the
//! axis the current shape would pick and the outcome is visible before the
//! press; a wrong guess costs one click on the new pane's close button, which
//! collapses the split again.

use iced::alignment::{Horizontal, Vertical};
use iced::widget::svg::Handle;
use iced::widget::{button, column, container, mouse_area, responsive, row, stack, svg, text};
use iced::{Element, Length};

use verse_core::{AlbumKey, NUM_BINS};

use crate::app::{DropTarget, Message};
use crate::artwork::Cache as ArtCache;
use crate::browsing::Context;
use crate::layout::{Axis, DropZone, PaneId, PaneMetrics};
use crate::pane::settings::PaneSettings;
use crate::pane::{
    PaneKind, PaneMessage, PaneState, artwork, collections, controls, library, queue, search,
    timeline, visualizer, volume,
};
use crate::styles::{self, PAD};
use crate::widgets::tooltip::tip;

const ROOT_BAND: f32 = 24.0;

const ICON_SIZE: f32 = 14.0;

const ICON_SPLIT_VERTICAL: &[u8] = include_bytes!("../../../assets/icons/split_vertical.svg");
const ICON_SPLIT_HORIZONTAL: &[u8] = include_bytes!("../../../assets/icons/split_horizontal.svg");
const ICON_CLOSE: &[u8] = include_bytes!("../../../assets/icons/close.svg");
const ICON_GRIP: &[u8] = include_bytes!("../../../assets/icons/grip.svg");
const ICON_SETTINGS: &[u8] = include_bytes!("../../../assets/icons/settings.svg");

#[derive(Clone, Copy)]
pub struct DragContext {
    pub active: bool,
    pub drop_zone: Option<DropZone>,
}

#[derive(Clone, Copy)]
pub struct Playback {
    pub is_playing: bool,
    pub position: f32,
    pub volume: f32,
    pub muted: bool,
    pub bins: [f32; NUM_BINS],
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            is_playing: false,
            position: 0.0,
            volume: 0.0,
            muted: false,
            bins: [0.0; NUM_BINS],
        }
    }
}

#[derive(Clone, Copy)]
pub struct Shared<'a> {
    pub playback: Playback,
    pub tracks: Context<'a>,
    pub visible: &'a [i64],
    pub visible_albums: &'a [AlbumKey],
    pub artwork: &'a ArtCache,
}

#[derive(Clone, Copy)]
pub struct Pane<'a> {
    pub id: PaneId,
    pub kind: PaneKind,
    pub state: Option<&'a PaneState>,
    pub settings: &'a PaneSettings,
}

#[allow(clippy::large_types_passed_by_value)]
pub fn view<'a>(
    pane: Pane<'a>,
    edit_mode: bool,
    drag: DragContext,
    shared: Shared<'a>,
    span: iced::Size,
) -> Element<'a, Message> {
    if !edit_mode {
        return content(pane, shared);
    }

    let mut layers = stack![content(pane, shared)];

    if let Some(zone) = drag.drop_zone {
        layers = layers.push(zone_highlight(zone));
    }

    if drag.active {
        layers = layers.push(hover_sensor(pane.id));
    } else {
        layers = layers.push(edit_overlay(pane.id, span));
    }

    layers.into()
}

#[allow(clippy::large_types_passed_by_value)]
fn content<'a>(pane: Pane<'a>, shared: Shared<'a>) -> Element<'a, Message> {
    match pane.kind {
        PaneKind::Controls => controls::view(shared.playback.is_playing),
        PaneKind::Library => library::view(shared.tracks, shared.visible),
        PaneKind::Queue => {
            static EMPTY: queue::State = queue::State::EMPTY;
            let state = match pane.state {
                Some(PaneState::Queue(state)) => state,
                _ => &EMPTY,
            };
            queue::view(
                shared.tracks,
                state,
                &queue::Bindings {
                    clear: Message::ClearQueue,
                },
            )
        }
        PaneKind::Timeline => {
            static EMPTY: timeline::State = timeline::State {
                show_remaining: false,
                hovered: None,
            };
            let state = match pane.state {
                Some(PaneState::Timeline(state)) => state,
                _ => &EMPTY,
            };
            timeline::view(
                shared.tracks,
                shared.playback.position,
                state,
                timeline::Bindings {
                    toggle_remaining: Message::Pane(
                        pane.id,
                        PaneMessage::Timeline(timeline::Message::ToggleRemaining),
                    ),
                    on_hover: Box::new(move |at| {
                        Message::Pane(
                            pane.id,
                            PaneMessage::Timeline(timeline::Message::Hovered(at)),
                        )
                    }),
                },
            )
        }
        // Unlike the other kinds, the fallback cannot be a `static`: this state
        // holds a handle behind a `RefCell`, which is not `Sync`. A pane without
        // state simply keeps no bridge, and draws the placeholder as it would
        // have anyway.
        PaneKind::Artwork => match pane.state {
            Some(PaneState::Artwork(state)) => {
                artwork::view(shared.tracks, shared.artwork, Some(state))
            }
            _ => artwork::view(shared.tracks, shared.artwork, None),
        },
        PaneKind::Collections => {
            static EMPTY: collections::State = collections::State::EMPTY;
            let state = match pane.state {
                Some(PaneState::Collections(state)) => state,
                _ => &EMPTY,
            };
            collections::view(
                shared.tracks,
                shared.visible_albums,
                shared.artwork,
                state,
                pane.id,
            )
        }
        PaneKind::Visualizer => {
            let settings = pane.settings.visualizer();
            let cover = settings
                .tint
                .needs_artwork()
                .then(|| {
                    // Nothing playing is a real answer rather than one still
                    // arriving, so the tint held across track changes is
                    // dropped here rather than outliving the last track.
                    let Some(id) = shared.tracks.playing else {
                        shared.artwork.forget_tint();
                        return None;
                    };

                    let track = shared.tracks.library.track(id)?;
                    shared.artwork.tint(id, track.path())
                })
                .flatten();

            visualizer::view(shared.playback.bins, settings, cover)
        }
        PaneKind::Volume => volume::view(shared.playback.volume, shared.playback.muted),
        PaneKind::Search => search::view(shared.tracks, shared.visible.len()),
        _ => container(text(pane.kind.title()).size(18))
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

fn edit_overlay<'a>(id: PaneId, span: iced::Size) -> Element<'a, Message> {
    responsive(move |pane_size| edit_controls(id, pane_size, span)).into()
}

fn edit_controls<'a>(id: PaneId, pane_size: iced::Size, span: iced::Size) -> Element<'a, Message> {
    let form = ControlForm::pick(pane_size);

    let buttons = [
        grab_handle(id),
        split_button(id, pane_size),
        options_button(id, pane_size, span),
        svg_button(ICON_CLOSE, "Close pane", Message::ClosePane(id)),
    ];

    let controls: Element<'a, Message> = match form {
        ControlForm::Row => row(buttons).spacing(GAP).into(),
        ControlForm::Vertical => column(buttons).spacing(GAP).into(),
    };

    container(
        container(controls)
            .padding(PAD / 2.0)
            .style(styles::floating_bar_style),
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
    Row,
    Vertical,
}

const BUTTON: f32 = ICON_SIZE + PAD * 2.0;
const GAP: f32 = PAD / 2.0;
const CHROME: f32 = PAD + PAD * 2.0;

const CONTROLS: f32 = 4.0;

impl ControlForm {
    fn pick(pane: iced::Size) -> Self {
        if pane.width >= Self::row_width() {
            Self::Row
        } else {
            Self::Vertical
        }
    }

    fn row_width() -> f32 {
        CONTROLS * BUTTON + (CONTROLS - 1.0) * GAP + CHROME
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

fn options_button<'a>(id: PaneId, pane_size: iced::Size, span: iced::Size) -> Element<'a, Message> {
    svg_button(
        ICON_SETTINGS,
        "Pane options",
        Message::OpenPaneOptions(
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
    tip(content, label).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Size;

    #[test]
    fn a_wide_pane_lays_the_controls_in_a_row() {
        let width = ControlForm::row_width();
        assert_eq!(ControlForm::pick(Size::new(width, 400.0)), ControlForm::Row);
    }

    #[test]
    fn a_wide_but_short_pane_still_uses_a_row() {
        let width = ControlForm::row_width();
        assert_eq!(ControlForm::pick(Size::new(width, 40.0)), ControlForm::Row);
    }

    #[test]
    fn too_narrow_for_four_icons_falls_back_to_vertical() {
        let width = ControlForm::row_width() - 1.0;
        assert_eq!(
            ControlForm::pick(Size::new(width, 400.0)),
            ControlForm::Vertical
        );
    }

    #[test]
    fn smallest_allowed_pane_uses_vertical() {
        let min = crate::layout::MIN_PANE;
        assert_eq!(
            ControlForm::pick(Size::new(min, min)),
            ControlForm::Vertical
        );
    }

    #[test]
    fn the_row_is_the_same_width_whatever_the_pane_holds() {
        let width = ControlForm::row_width();
        for kind in PaneKind::ALL {
            assert_eq!(
                ControlForm::pick(Size::new(width, 400.0)),
                ControlForm::Row,
                "{kind:?} did not take the row at its exact width"
            );
            assert_eq!(
                ControlForm::pick(Size::new(width - 1.0, 400.0)),
                ControlForm::Vertical,
                "{kind:?} did not stack just below the row width"
            );
        }
    }

    #[test]
    fn dropping_a_control_did_not_make_the_row_wider() {
        assert!(
            ControlForm::row_width() < 5.0 * BUTTON + 4.0 * GAP + CHROME,
            "four controls should need less width than the five that preceded them"
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
    fn the_vertical_stack_overflows_the_smallest_panes() {
        let needed = ControlForm::vertical_height();
        assert!(
            needed > crate::layout::MIN_PANE,
            "the stack fits the floor now; escaping controls may be unnecessary"
        );
    }

    #[test]
    fn a_pane_tall_enough_for_content_still_may_not_fit_the_controls() {
        let strip = 40.0;
        assert!(
            ControlForm::vertical_height() > strip,
            "a {strip}px strip fits the control stack, so nothing escapes"
        );
    }
}
