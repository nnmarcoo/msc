//! The volume pane: a mute button, a rail, and the level.
//!
//! One row, in the order the gesture is made: a mute button, the rail, and the
//! level as a percentage.
//!
//! The pane owns the row. [`crate::widgets::volume`] is only the rail, because
//! only the rail needs to map a pointer to a level along its width; the icon is
//! a button and the readout is a label, so composing them here keeps the rail a
//! widget any other pane could take.
//!
//! [`Form`] drops parts of the row as the pane narrows: the readout goes first,
//! since the rail's fill already says roughly how loud, then the icon, which
//! outlasts it because muting is a control rather than a description. Every form
//! keeps the rail. The readout reads above 100% when boosting, since 100% is the
//! file as mastered and not the top of the scale.
//!
//! `LOUD` is half of unity rather than half the rail, because the glyph is about
//! loudness relative to the recording and everything from unity up is loud. A
//! level of zero draws the muted glyph whether or not the mute is on: what the
//! icon reports is whether sound is coming out, and those two are the same to a
//! listener.
//!
//! Muting is the app's, not the pane's: [`crate::app`] keeps the flag and hands
//! this pane the level actually reaching the speakers, zero while muted. So the
//! rail has one meaning rather than a level and a flag that can disagree, and
//! dragging while muted unmutes. The pane is
//! [`crate::pane::PaneState::Stateless`] for the same reason: two volume panes
//! must agree about the volume, so there is nothing here to disagree about.

use iced::widget::svg::Handle;
use iced::widget::tooltip::Position;
use iced::widget::{button, container, responsive, row, svg, text, tooltip};
use iced::{Element, Length};

use crate::app::Message;
use crate::styles::{self, LABEL_FONT_SIZE, PAD, TOOLTIP_DELAY};
use crate::widgets::volume::{Op, Volume, percent};

const ICON_HIGH: &[u8] = include_bytes!("../../../assets/icons/volume_high.svg");
const ICON_LOW: &[u8] = include_bytes!("../../../assets/icons/volume_low.svg");
const ICON_MUTED: &[u8] = include_bytes!("../../../assets/icons/volume_muted.svg");

const ICON_SIZE: f32 = 16.0;
const GAP: f32 = PAD * 1.5;

const LOUD: f32 = 0.5;
const READOUT_WIDTH: f32 = 32.0;

pub fn view<'a>(level: f32, muted: bool) -> Element<'a, Message> {
    responsive(move |size| body(level, muted, Form::pick(size))).into()
}

fn body<'a>(level: f32, muted: bool, form: Form) -> Element<'a, Message> {
    let mut line = row![].spacing(GAP).align_y(iced::Center);

    if form.shows_icon() {
        line = line.push(mute_button(level, muted));
    }

    line = line.push(Volume::new(level, |op| match op {
        Op::Set(level) => Message::Volume(level),
        Op::Committed => Message::SaveConfig,
    }));

    if form.shows_readout() {
        line = line.push(readout(level));
    }

    container(line)
        .padding(PAD)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    Full,
    NoReadout,
    RailOnly,
}

impl Form {
    fn pick(pane: iced::Size) -> Self {
        if pane.width >= Self::full_width() {
            Self::Full
        } else if pane.width >= Self::no_readout_width() {
            Self::NoReadout
        } else {
            Self::RailOnly
        }
    }

    fn shows_icon(self) -> bool {
        self != Self::RailOnly
    }

    fn shows_readout(self) -> bool {
        self == Self::Full
    }

    fn icon_button_width() -> f32 {
        ICON_SIZE + PAD * 2.0
    }

    fn chrome() -> f32 {
        PAD * 2.0
    }

    fn no_readout_width() -> f32 {
        Self::icon_button_width() + GAP + Volume::<Message>::min_width() + Self::chrome()
    }

    fn full_width() -> f32 {
        Self::no_readout_width() + GAP + READOUT_WIDTH
    }
}

fn glyph_for(level: f32, muted: bool) -> (&'static [u8], &'static str) {
    if muted || level <= 0.0 {
        (ICON_MUTED, "Unmute")
    } else if level > LOUD {
        (ICON_HIGH, "Mute")
    } else {
        (ICON_LOW, "Mute")
    }
}

fn mute_button<'a>(level: f32, muted: bool) -> Element<'a, Message> {
    let (icon, label) = glyph_for(level, muted);

    let glyph = svg(Handle::from_memory(icon))
        .style(styles::svg_style)
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE));

    let control = button(glyph)
        .on_press(Message::ToggleMute)
        .padding(PAD)
        .style(styles::icon_button_style);

    tooltip(
        control,
        container(text(label).size(LABEL_FONT_SIZE))
            .padding(PAD)
            .style(styles::tooltip_style),
        Position::Top,
    )
    .delay(TOOLTIP_DELAY)
    .gap(PAD)
    .into()
}

fn readout<'a>(level: f32) -> Element<'a, Message> {
    container(
        text(percent(level))
            .size(LABEL_FONT_SIZE)
            .style(|theme: &iced::Theme| text::Style {
                color: Some(
                    theme
                        .extended_palette()
                        .background
                        .base
                        .text
                        .scale_alpha(0.7),
                ),
            })
            .align_x(iced::alignment::Horizontal::Right),
    )
    .width(Length::Fixed(READOUT_WIDTH))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::MIN_PANE;
    use iced::Size;

    #[test]
    fn a_wide_pane_shows_the_whole_row() {
        assert_eq!(Form::pick(Size::new(300.0, 60.0)), Form::Full);
    }

    #[test]
    fn the_readout_goes_before_the_icon_does() {
        let width = Form::full_width() - 1.0;
        let form = Form::pick(Size::new(width, 60.0));

        assert_eq!(form, Form::NoReadout);
        assert!(form.shows_icon(), "the icon should outlast the readout");
        assert!(!form.shows_readout());
    }

    #[test]
    fn a_pane_too_narrow_for_the_icon_keeps_the_rail() {
        let width = Form::no_readout_width() - 1.0;
        assert_eq!(Form::pick(Size::new(width, 60.0)), Form::RailOnly);
    }

    #[test]
    fn every_form_keeps_the_rail() {
        for width in [0.0, MIN_PANE, Form::no_readout_width(), 1_000.0] {
            let form = Form::pick(Size::new(width, 60.0));
            assert!(
                matches!(form, Form::Full | Form::NoReadout | Form::RailOnly),
                "width {width} gave a form with no rail"
            );
        }
    }

    #[test]
    fn the_forms_degrade_in_width_order() {
        assert!(Form::full_width() > Form::no_readout_width());
    }

    #[test]
    fn the_smallest_allowed_pane_still_shows_something() {
        let form = Form::pick(Size::new(MIN_PANE, MIN_PANE));
        assert!(
            !form.shows_readout(),
            "80px cannot fit icon, rail, and 100%"
        );
    }

    #[test]
    fn each_threshold_takes_its_own_form_at_its_exact_width() {
        assert_eq!(Form::pick(Size::new(Form::full_width(), 60.0)), Form::Full);
        assert_eq!(
            Form::pick(Size::new(Form::no_readout_width(), 60.0)),
            Form::NoReadout
        );
    }

    #[test]
    fn the_row_fits_the_width_its_form_asks_for() {
        let needed = Form::full_width();
        let content = Form::icon_button_width()
            + GAP
            + Volume::<Message>::min_width()
            + GAP
            + READOUT_WIDTH
            + Form::chrome();

        assert!(
            content <= needed,
            "the full row needs {content} but claims to fit in {needed}"
        );
    }

    #[test]
    fn a_silent_player_reads_as_muted_whether_or_not_it_is() {
        assert_eq!(glyph_for(0.0, false).0, ICON_MUTED);
        assert_eq!(glyph_for(0.0, true).0, ICON_MUTED);
    }

    #[test]
    fn muting_does_not_depend_on_the_level_behind_it() {
        assert_eq!(glyph_for(0.9, true).0, ICON_MUTED);
    }

    #[test]
    fn the_glyph_follows_how_loud_it_is() {
        assert_eq!(glyph_for(0.2, false).0, ICON_LOW);
        assert_eq!(glyph_for(LOUD, false).0, ICON_LOW);
        assert_eq!(glyph_for(0.8, false).0, ICON_HIGH);
    }

    #[test]
    fn the_tooltip_offers_the_action_not_the_state() {
        assert_eq!(glyph_for(0.7, false).1, "Mute");
        assert_eq!(glyph_for(0.7, true).1, "Unmute");
        assert_eq!(
            glyph_for(0.0, false).1,
            "Unmute",
            "a slider dragged to zero should still offer to restore the level"
        );
    }
}
