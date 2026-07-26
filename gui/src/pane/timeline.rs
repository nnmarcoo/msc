//! The timeline pane: what is playing, a seek bar, and the clocks.
//!
//! Three rows, tight against each other: the title, artist and rating; the seek
//! rail; then the elapsed and total clocks at either end beneath it.
//!
//! The pane owns all three. [`crate::widgets::timeline`] is only the rail,
//! because only the rail needs to map a pointer to a position along its width;
//! everything on the rows above and below is a label or a
//! [`crate::widgets::rating`], and composing them here keeps the stars a widget
//! any other pane can use, so a library row wanting to show a rating takes the
//! same one rather than a reimplementation.
//!
//! The rows carry no spacing of their own. An earlier arrangement gave the rail
//! its own reserved lines for the clocks *and* let the pane stack its rows
//! outside them, which left the title floating some twenty pixels clear of the
//! bar it belonged to. The rail is a thin thing that has to read as attached to
//! its labels, so the gap between them is the widget's own seek margin and
//! nothing more.
//!
//! The rows group by what the reader is asking. Above the rail is what is
//! playing: its name, and the rating that judges it. Below is where in it
//! playback has got to, elapsed and total at either end of the bar they measure,
//! each sitting at the end of the rail it refers to.
//!
//! `hovered` is the position a pointer is aiming at, reported by the rail and
//! held here so the elapsed readout can show the time under the cursor before a
//! seek is committed. It is pane state rather than widget state because the
//! label that displays it is the pane's; the rail keeps its own copy for the
//! ghost head, since that redraws without re-running `view`.
//!
//! `show_remaining` is per-pane state for the same reason: it is about how this
//! pane draws its right-hand clock and nothing else, so two timeline panes are
//! entitled to disagree about it. See [`crate::pane::PaneState`].
//!
//! Title, artist, and rating come from the *playing* track rather than the
//! queue's current one, so the rows always describe the audio actually coming
//! out. The two diverge for as long as a queued track has yet to start. That
//! also decides what a click on the stars rates: the track being heard.
//!
//! Position comes from the player each frame, except while a drag is in flight:
//! `seeking` on the app then holds the pointer's position and the rail reads
//! that instead, so the bar cannot stutter against a player still reporting
//! where it was a moment ago.
//!
//! The audio moves once, on release, since kira restarts the stream at every
//! seek and doing so per pointer-move makes a drag audibly stutter. What made
//! deferral feel laggy before was not the deferral but the handoff back to the
//! player afterwards; see `settle_seek` in [`crate::app`].
//!
//! With nothing playing the bar still draws, empty and inert, so switching tracks
//! does not make the pane change shape. The title is blank rather than a
//! placeholder, since a label naming the absence is noise next to a rail that
//! already shows it, and an empty string holds the row's height either way. The
//! stars go with the track, so they are hidden rather than drawn as five outlines
//! that would do nothing when clicked.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Length};

use crate::app::Message as AppMessage;
use crate::styles::{LABEL_FONT_SIZE, PAD};
use crate::tracks::Context;
use crate::widgets::rating::Rating;
use crate::widgets::timeline::{Op, Timeline, clock};

const TITLE_FONT_SIZE: f32 = 13.0;
const ROW_GAP: f32 = 6.0;

#[derive(Debug, Default)]
pub struct State {
    pub show_remaining: bool,
    pub hovered: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleRemaining,
    Hovered(Option<f32>),
}

pub fn update(state: &mut State, message: &Message) {
    match message {
        Message::ToggleRemaining => state.show_remaining = !state.show_remaining,
        Message::Hovered(position) => state.hovered = *position,
    }
}

pub struct Bindings<'a> {
    pub toggle_remaining: AppMessage,
    pub on_hover: Box<dyn Fn(Option<f32>) -> AppMessage + 'a>,
}

pub fn view<'a>(
    tracks: Context<'a>,
    position: f32,
    state: &State,
    bindings: Bindings<'a>,
) -> Element<'a, AppMessage> {
    let track = tracks.playing.and_then(|id| tracks.library.track(id));
    let duration = track.map_or(0.0, verse_core::Track::duration);

    let on_hover = bindings.on_hover;
    let bar = Timeline::new(position, duration, move |op| match op {
        Op::Seek(seconds) => AppMessage::Seek(seconds),
        Op::Committed => AppMessage::SeekReleased,
        Op::Hovered(at) => on_hover(at),
    });

    let shown = state.hovered.unwrap_or(position);
    let aiming = state.hovered.is_some();

    let rows = column![
        row![
            name(track),
            Space::new().width(Length::Fill),
            stars(tracks, track),
        ]
        .align_y(iced::Center),
        bar,
        row![
            elapsed(shown, aiming),
            Space::new().width(Length::Fill),
            remaining(
                duration,
                shown,
                state.show_remaining,
                bindings.toggle_remaining
            ),
        ]
        .align_y(iced::Center),
    ];

    container(rows)
        .padding(PAD)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn name(track: Option<&verse_core::Track>) -> Element<'_, AppMessage> {
    let title = track.and_then(verse_core::Track::title).unwrap_or_default();

    let mut line = row![
        text(title)
            .size(TITLE_FONT_SIZE)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            })
            .style(|theme: &iced::Theme| text::Style {
                color: Some(shade(theme, true)),
            })
    ]
    .spacing(ROW_GAP)
    .align_y(iced::Center);

    if let Some(artist) = track.and_then(verse_core::Track::track_artist) {
        line = line.push(dim(text(artist).size(LABEL_FONT_SIZE)));
    }

    line.into()
}

fn stars<'a>(tracks: Context<'a>, track: Option<&'a verse_core::Track>) -> Element<'a, AppMessage> {
    match tracks.playing.filter(|_| track.is_some()) {
        Some(id) => Rating::new(track.and_then(verse_core::Track::rating))
            .on_rate(move |value| AppMessage::RateTrack(id, value))
            .into(),
        None => Space::new().into(),
    }
}

fn elapsed<'a>(shown: f32, aiming: bool) -> Element<'a, AppMessage> {
    let readout = text(clock(shown))
        .size(LABEL_FONT_SIZE)
        .style(move |theme: &iced::Theme| text::Style {
            color: Some(shade(theme, aiming)),
        });

    readout.into()
}

fn remaining<'a>(
    duration: f32,
    shown: f32,
    counting_down: bool,
    on_press: AppMessage,
) -> Element<'a, AppMessage> {
    let content = if counting_down && duration > 0.0 {
        format!("-{}", clock((duration - shown).max(0.0)))
    } else {
        clock(duration)
    };

    button(dim(text(content).size(LABEL_FONT_SIZE)))
        .on_press(on_press)
        .padding(0)
        .style(|_theme, _status| button::Style::default())
        .into()
}

fn dim(label: text::Text<'_, iced::Theme>) -> Element<'_, AppMessage> {
    label
        .style(|theme: &iced::Theme| text::Style {
            color: Some(shade(theme, false)),
        })
        .into()
}

fn shade(theme: &iced::Theme, strong: bool) -> iced::Color {
    let base = theme.extended_palette().background.base.text;
    if strong { base } else { base.scale_alpha(0.7) }
}
