//! Transport controls: previous, play/pause, next.
//!
//! A pane can be any size the layout gives it, so [`Metrics`] sizes the buttons
//! from the space available rather than from a fixed idea of the shape.
//!
//! # Which way the buttons run
//!
//! A transport is a row by habit rather than necessity, and a pane dragged tall
//! and narrow is a column-shaped hole: laying a row into it wastes the height and
//! starves the width. [`Axis::of`] turns the buttons to face the long side, so
//! previous/next read left-to-right or top-to-bottom accordingly.
//!
//! `TURN_RATIO` is the margin by which height must beat width before the column
//! is worth it. It is above 1.0 so a square-ish pane keeps its row and a pane
//! dragged along its diagonal does not flicker between the two.
//!
//! # What fits, and what goes when it does not
//!
//! All three buttons are the pane: a transport that cannot skip is not a
//! transport, so they shrink long before any is dropped. The cross axis proposes
//! an icon size and the main axis disposes of it — [`Metrics::pick`] takes what
//! the cross affords, then solves for the largest icon the main axis can hold if
//! three at that size would overrun it. The inverse is a division rather than a
//! search because the extent three buttons need is linear in the icon size, and
//! the same arithmetic serves both axes because a button is square.
//!
//! Spacing and padding scale with the icon. Fixed ones are most of a narrow
//! pane's extent, and charging full price would crush the icons paying for them.
//!
//! Only below `ICON_FLOOR`, where three buttons would be too small to aim at, do
//! they give up on the pair and draw play/pause alone. The floor sits under what
//! [`crate::layout::MIN_PANE`] affords, so that is unreachable through the layout
//! — it keeps the pane honest at any size rather than only at today's.

use iced::widget::svg::Handle;
use iced::widget::{Column, Row, button, container, responsive, svg};
use iced::{Element, Length};

use crate::app::Message;
use crate::styles::{self, PAD};
use crate::widgets::tooltip::tip;

const ICON_PLAY: &[u8] = include_bytes!("../../../assets/icons/play.svg");
const ICON_PAUSE: &[u8] = include_bytes!("../../../assets/icons/pause.svg");
const ICON_NEXT: &[u8] = include_bytes!("../../../assets/icons/next.svg");
const ICON_PREVIOUS: &[u8] = include_bytes!("../../../assets/icons/previous.svg");

const ICON_MIN: f32 = 20.0;
const ICON_MAX: f32 = 48.0;
const HEIGHT_SHARE: f32 = 0.45;
const PRIMARY_SCALE: f32 = 1.4;

const ICON_FLOOR: f32 = 6.0;

const SPACING_SHARE: f32 = 0.35;

const TURN_RATIO: f32 = 1.25;

const PADDING_SHARE: f32 = 0.2;

pub fn view<'a>(is_playing: bool) -> Element<'a, Message> {
    responsive(move |size| transport(is_playing, Metrics::pick(size))).into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    Row,
    Column,
}

impl Axis {
    fn of(pane: iced::Size) -> Self {
        if pane.height > pane.width * TURN_RATIO {
            Self::Column
        } else {
            Self::Row
        }
    }

    fn main(self, pane: iced::Size) -> f32 {
        match self {
            Self::Row => pane.width,
            Self::Column => pane.height,
        }
    }

    fn cross(self, pane: iced::Size) -> f32 {
        match self {
            Self::Row => pane.height,
            Self::Column => pane.width,
        }
    }
}

fn transport<'a>(is_playing: bool, metrics: Metrics) -> Element<'a, Message> {
    let (play_icon, play_label) = if is_playing {
        (ICON_PAUSE, "Pause")
    } else {
        (ICON_PLAY, "Play")
    };

    let padding = metrics.padding();
    let play_pause = icon_button(
        play_icon,
        play_label,
        Message::PlayPause,
        metrics.primary_icon(),
        padding,
    );

    let buttons = if metrics.full {
        vec![
            icon_button(
                ICON_PREVIOUS,
                "Previous",
                Message::Previous,
                metrics.icon,
                padding,
            ),
            play_pause,
            icon_button(ICON_NEXT, "Next", Message::Next, metrics.icon, padding),
        ]
    } else {
        vec![play_pause]
    };

    let controls: Element<'_, Message> = match metrics.axis {
        Axis::Row => Row::from_vec(buttons)
            .spacing(metrics.spacing)
            .align_y(iced::Center)
            .into(),
        Axis::Column => Column::from_vec(buttons)
            .spacing(metrics.spacing)
            .align_x(iced::Center)
            .into(),
    };

    container(controls)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(padding)
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Metrics {
    icon: f32,
    spacing: f32,
    full: bool,
    axis: Axis,
}

impl Metrics {
    fn pick(pane: iced::Size) -> Self {
        let axis = Axis::of(pane);
        let wanted = Self::icon_for_cross(axis.cross(pane));
        let afforded = Self::icon_for_main(axis.main(pane));
        let icon = wanted.min(afforded);

        if icon >= ICON_FLOOR {
            return Self {
                icon,
                spacing: Self::spacing_for(icon),
                full: true,
                axis,
            };
        }

        Self {
            icon: wanted,
            spacing: Self::spacing_for(wanted),
            full: false,
            axis,
        }
    }

    fn icon_for_cross(cross: f32) -> f32 {
        (cross * HEIGHT_SHARE).clamp(ICON_MIN, ICON_MAX) / PRIMARY_SCALE
    }

    fn icon_for_main(main: f32) -> f32 {
        (main.max(0.0) / Self::extent_per_icon()).min(ICON_MAX / PRIMARY_SCALE)
    }

    fn spacing_for(icon: f32) -> f32 {
        icon * SPACING_SHARE
    }

    fn primary_icon(self) -> f32 {
        self.icon * PRIMARY_SCALE
    }

    fn padding(self) -> f32 {
        Self::padding_for(self.icon)
    }

    fn padding_for(icon: f32) -> f32 {
        (icon * PADDING_SHARE).min(PAD)
    }

    fn extent_per_icon() -> f32 {
        2.0 + PRIMARY_SCALE + PADDING_SHARE * 8.0 + SPACING_SHARE * 2.0
    }

    fn full_extent(icon: f32) -> f32 {
        let padding = Self::padding_for(icon);
        let button = |size: f32| size + padding * 2.0;

        button(icon) * 2.0
            + button(icon * PRIMARY_SCALE)
            + Self::spacing_for(icon) * 2.0
            + padding * 2.0
    }
}

fn icon_button<'a>(
    bytes: &'static [u8],
    label: &'a str,
    message: Message,
    size: f32,
    padding: f32,
) -> Element<'a, Message> {
    let glyph = svg(Handle::from_memory(bytes))
        .style(styles::svg_style)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size));

    let control = button(glyph)
        .on_press(message)
        .padding(padding)
        .style(styles::icon_button_style);

    tip(control, label).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::MIN_PANE;
    use iced::Size;

    #[test]
    fn wide_pane_shows_all_three_buttons() {
        assert!(Metrics::pick(Size::new(400.0, 80.0)).full);
    }

    #[test]
    fn the_smallest_allowed_pane_still_shows_all_three() {
        assert!(
            Metrics::pick(Size::new(MIN_PANE, MIN_PANE)).full,
            "the transport lost its skip buttons at a size the layout can hand it"
        );
    }

    #[test]
    fn icon_never_leaves_the_readable_range() {
        for height in [0.0, MIN_PANE, 200.0, 2000.0] {
            let primary = Metrics::pick(Size::new(400.0, height)).primary_icon();
            assert!(
                (ICON_MIN..=ICON_MAX).contains(&primary),
                "height {height} gave primary icon {primary}"
            );
        }
    }

    fn panes() -> impl Iterator<Item = Size> {
        let extents = || (0..40).map(|step| MIN_PANE + step as f32 * 25.0);
        extents().flat_map(move |w| extents().map(move |h| Size::new(w, h)))
    }

    #[test]
    fn the_buttons_fit_the_pane_they_are_given() {
        for pane in panes() {
            let metrics = Metrics::pick(pane);
            if !metrics.full {
                continue;
            }

            let needed = Metrics::full_extent(metrics.icon);
            let along = metrics.axis.main(pane);
            assert!(
                needed <= along + 0.001,
                "a {pane:?} pane laid {:?} needed {needed} along {along}",
                metrics.axis
            );
        }
    }

    #[test]
    fn the_buttons_fit_across_the_pane_too() {
        for pane in panes() {
            let metrics = Metrics::pick(pane);
            let thickest = metrics.primary_icon() + metrics.padding() * 4.0;
            let across = metrics.axis.cross(pane);
            assert!(
                thickest <= across + 0.001,
                "a {pane:?} pane needed {thickest} across {across}"
            );
        }
    }

    #[test]
    fn icon_grows_with_pane_height() {
        let short = Metrics::pick(Size::new(400.0, MIN_PANE)).icon;
        let medium = Metrics::pick(Size::new(400.0, 100.0)).icon;
        let tall = Metrics::pick(Size::new(400.0, 300.0)).icon;
        assert!(medium > short, "{medium} should exceed {short}");
        assert!(tall > medium, "{tall} should exceed {medium}");
    }

    #[test]
    fn a_narrow_pane_shrinks_the_icons_rather_than_dropping_them() {
        let tall = 400.0;
        let roomy = Metrics::pick(Size::new(400.0, tall));
        let narrow = Metrics::pick(Size::new(90.0, tall));

        assert!(roomy.full && narrow.full);
        assert!(
            narrow.icon < roomy.icon,
            "the same icon was drawn in a pane a quarter the width"
        );
    }

    #[test]
    fn width_only_ever_takes_size_away() {
        let mut last = Metrics::pick(Size::new(1000.0, 400.0)).icon;

        for step in (0..1000).rev() {
            let metrics = Metrics::pick(Size::new(step as f32, 400.0));
            if !metrics.full {
                break;
            }

            assert!(
                metrics.icon <= last + 0.001,
                "the icon grew as the pane narrowed to {step}"
            );
            last = metrics.icon;
        }
    }

    fn cramped() -> Size {
        Size::new(20.0, 20.0)
    }

    #[test]
    fn dropping_the_pair_gives_play_pause_the_room_it_freed() {
        let alone = Metrics::pick(cramped());
        let trio = Metrics::pick(Size::new(MIN_PANE, MIN_PANE));

        assert!(!alone.full && trio.full);
        assert!(
            alone.primary_icon() > trio.primary_icon(),
            "play/pause stayed cramped though it no longer shares the pane"
        );
    }

    #[test]
    fn a_pane_too_small_for_three_keeps_play_pause() {
        let metrics = Metrics::pick(cramped());
        assert!(!metrics.full);
        assert!(metrics.primary_icon() >= ICON_MIN);
    }

    #[test]
    fn turning_rescues_a_pane_a_row_would_have_given_up_on() {
        let sliver = Size::new(30.0, 400.0);

        assert!(
            Metrics::icon_for_main(sliver.width) < ICON_FLOOR,
            "this pane is wide enough for a row; it proves nothing"
        );
        assert!(
            Metrics::pick(sliver).full,
            "a pane with 400px of height to lay buttons along still dropped them"
        );
    }

    #[test]
    fn the_floor_is_below_anything_the_layout_can_reach() {
        let smallest = Metrics::icon_for_main(MIN_PANE);
        assert!(
            smallest >= ICON_FLOOR,
            "the smallest pane affords {smallest}, under the {ICON_FLOOR} floor"
        );
    }

    #[test]
    fn solving_never_asks_for_more_than_it_was_given() {
        for step in 0..600 {
            let main = step as f32;
            let icon = Metrics::icon_for_main(main);
            let needed = Metrics::full_extent(icon);

            assert!(
                needed <= main + 0.001,
                "solving {main} gave an icon wanting {needed}"
            );
        }
    }

    #[test]
    fn the_solve_is_exact_while_the_padding_is_uncapped() {
        for main in [45.0, 60.0, 75.0] {
            let icon = Metrics::icon_for_main(main);
            assert!(
                Metrics::padding_for(icon) < PAD,
                "{main} is long enough to cap the padding; pick a tighter case"
            );
            assert!(
                (Metrics::full_extent(icon) - main).abs() < 0.001,
                "the solve left {main} short of exact"
            );
        }
    }

    #[test]
    fn a_pane_long_in_its_main_axis_is_sized_by_the_cross_one() {
        let wide = Metrics::pick(Size::new(4000.0, 120.0));
        assert_eq!(wide.axis, Axis::Row);
        assert!((wide.icon - Metrics::icon_for_cross(120.0)).abs() < 0.001);

        let tall = Metrics::pick(Size::new(120.0, 4000.0));
        assert_eq!(tall.axis, Axis::Column);
        assert!((tall.icon - Metrics::icon_for_cross(120.0)).abs() < 0.001);
    }

    #[test]
    fn play_pause_is_the_largest_button() {
        let metrics = Metrics::pick(Size::new(400.0, 80.0));
        assert!(metrics.primary_icon() > metrics.icon);
    }

    #[test]
    fn the_buttons_face_the_long_side() {
        assert_eq!(Axis::of(Size::new(400.0, 60.0)), Axis::Row);
        assert_eq!(Axis::of(Size::new(60.0, 400.0)), Axis::Column);
    }

    #[test]
    fn a_squarish_pane_keeps_its_row() {
        for extent in [MIN_PANE, 80.0, 200.0, 600.0] {
            assert_eq!(
                Axis::of(Size::new(extent, extent)),
                Axis::Row,
                "a square {extent} pane turned when it had no reason to"
            );
        }
    }

    #[test]
    fn turning_needs_a_margin_rather_than_a_hair() {
        let width = 100.0;
        assert_eq!(Axis::of(Size::new(width, width * 1.1)), Axis::Row);
        assert_eq!(Axis::of(Size::new(width, width * 1.4)), Axis::Column);
    }

    #[test]
    fn a_tall_pane_uses_its_height_rather_than_starving_on_width() {
        let pane = Size::new(70.0, 400.0);
        let upright = Metrics::pick(pane);

        assert_eq!(upright.axis, Axis::Column);
        assert!(upright.full);
        assert!(
            upright.icon > Metrics::icon_for_main(pane.width),
            "the column was still sized by the width it does not lay along"
        );
    }

    #[test]
    fn a_tall_pane_beats_the_row_it_replaced() {
        let pane = Size::new(70.0, 400.0);
        let column = Metrics::pick(pane);
        let as_a_row = Metrics::icon_for_main(pane.width).min(Metrics::icon_for_cross(pane.height));

        assert!(
            column.icon > as_a_row,
            "turning the buttons gained nothing: {} against {as_a_row}",
            column.icon
        );
    }

    #[test]
    fn every_pane_the_layout_can_hand_us_keeps_all_three_either_way() {
        for pane in panes() {
            assert!(
                Metrics::pick(pane).full,
                "{pane:?} dropped the pair despite room to turn"
            );
        }
    }
}
