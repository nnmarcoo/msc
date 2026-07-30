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
//!
//! # What fits, and what goes when it does not
//!
//! [`Form`] answers three questions of a pane's width: whether the stars, the
//! artist, and the clocks are drawn. Each part answers only for the room it needs,
//! so nothing is dropped on another's account — in particular the clocks sit on
//! their own row beneath the rail and compete with nothing, so a long name cannot
//! take them down with it.
//!
//! The stars go exactly when keeping them would start cutting the name, which
//! means the decision has to know how wide the name actually is. That is measured
//! from the strings via [`crate::widgets::marquee::width_of`], not assumed: how
//! much room a name wants is a property of the text, and a fixed "titles are
//! usually this wide" guess is wrong in both directions at once, holding the stars
//! through the truncation of a long name while dropping them beside a short one
//! that had room to spare. Breakpoints therefore differ per track, which is the
//! intent rather than a fault.
//!
//! The artist is cut rather than dropped for as long as anything can be seen of
//! it, since a truncated artist still says more than an absent one. There is no
//! minimum width per label: a floor is a demand dressed as a limit, and charging
//! one throws the measurement away, because every name longer than the floor then
//! asks for the same width and the artist vanishes at one pane size whatever the
//! track.
//!
//! Measuring is not free and `view` runs at [`crate::app`]'s 16ms tick while a
//! track plays, so the widths are memoised against the playing track's id and
//! recomputed only when the track changes. Nothing edits a track's tags in place,
//! so the id is a sound key for the strings.

use std::rc::Rc;

use iced::widget::{Space, button, column, container, responsive, row, text};
use iced::{Element, Length};

use crate::app::Message as AppMessage;
use crate::browsing::Context;
use crate::styles::{LABEL_FONT_SIZE, PAD};
use crate::widgets::marquee::{self, marquee};
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
    let show_remaining = state.show_remaining;
    let hovered = state.hovered;

    let toggle = bindings.toggle_remaining;
    let on_hover: Rc<dyn Fn(Option<f32>) -> AppMessage + 'a> = Rc::from(bindings.on_hover);

    let name_width = NameWidth::measured(
        tracks.playing,
        tracks.playing.and_then(|id| tracks.library.track(id)),
    );

    responsive(move |size| {
        body(
            tracks,
            position,
            &State {
                show_remaining,
                hovered,
            },
            toggle.clone(),
            &on_hover,
            Form::pick(size.width, name_width),
        )
    })
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Form {
    stars: bool,
    artist: bool,
    clocks: bool,
}

impl Form {
    fn pick(width: f32, name: NameWidth) -> Self {
        let content = width - chrome();
        let stars = Rating::<AppMessage>::width_for(STAR_SIZE, STAR_SPACING) + ROW_GAP;

        Self {
            stars: content >= name.wanted() + stars,
            artist: name.has_artist() && content >= ROW_GAP + SLIVER * 2.0,
            clocks: content >= CLOCKS_FLOOR,
        }
    }
}

fn chrome() -> f32 {
    PAD * 2.0
}

#[derive(Debug, Clone, Copy, Default)]
struct NameWidth {
    title: f32,
    artist: f32,
}

impl NameWidth {
    fn measured(id: Option<i64>, track: Option<&verse_core::Track>) -> Self {
        thread_local! {
            static LAST: std::cell::Cell<Option<(Option<i64>, NameWidth)>> =
                const { std::cell::Cell::new(None) };
        }

        LAST.with(|last| {
            if let Some((was, width)) = last.get()
                && was == id
            {
                return width;
            }

            let width = Self::of(track);
            last.set(Some((id, width)));
            width
        })
    }

    fn of(track: Option<&verse_core::Track>) -> Self {
        let title = track.and_then(verse_core::Track::title).unwrap_or_default();
        let artist = track.and_then(verse_core::Track::track_artist);

        Self {
            title: marquee::width_of(title, TITLE_FONT_SIZE, TITLE_FONT),
            artist: artist.map_or(0.0, |artist| {
                marquee::width_of(artist, LABEL_FONT_SIZE, iced::Font::DEFAULT)
            }),
        }
    }

    fn has_artist(self) -> bool {
        self.artist > 0.0
    }

    fn wanted(self) -> f32 {
        if self.has_artist() {
            self.title + ROW_GAP + self.artist
        } else {
            self.title
        }
    }
}

const SLIVER: f32 = 24.0;

const CLOCKS_FLOOR: f32 = 96.0;

const TITLE_FONT: iced::Font = iced::Font {
    weight: iced::font::Weight::Bold,
    ..iced::Font::DEFAULT
};

const STAR_SIZE: f32 = 13.0;
const STAR_SPACING: f32 = 2.0;

fn body<'a>(
    tracks: Context<'a>,
    position: f32,
    state: &State,
    toggle_remaining: AppMessage,
    on_hover: &Rc<dyn Fn(Option<f32>) -> AppMessage + 'a>,
    form: Form,
) -> Element<'a, AppMessage> {
    let track = tracks.playing.and_then(|id| tracks.library.track(id));
    let duration = track.map_or(0.0, verse_core::Track::duration);

    let on_hover = Rc::clone(on_hover);
    let bar = Timeline::new(position, duration, move |op| match op {
        Op::Seek(seconds) => AppMessage::Seek(seconds),
        Op::Committed => AppMessage::SeekReleased,
        Op::Hovered(at) => on_hover(at),
    });

    let shown = state.hovered.unwrap_or(position);
    let aiming = state.hovered.is_some();

    let mut top = row![name(track, form)]
        .spacing(ROW_GAP)
        .align_y(iced::Center);
    if form.stars {
        top = top.push(stars(tracks, track));
    }

    let mut rows = column![top, bar];

    if form.clocks {
        rows = rows.push(
            row![
                elapsed(shown, aiming),
                Space::new().width(Length::Fill),
                remaining(duration, shown, state.show_remaining, toggle_remaining),
            ]
            .align_y(iced::Center),
        );
    }

    container(rows)
        .padding(PAD)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn name(track: Option<&verse_core::Track>, form: Form) -> Element<'_, AppMessage> {
    let title = track.and_then(verse_core::Track::title).unwrap_or_default();

    let mut line = row![
        marquee(title)
            .size(TITLE_FONT_SIZE)
            .font(TITLE_FONT)
            .style(|theme: &iced::Theme| shade(theme, true))
    ]
    .spacing(ROW_GAP)
    .align_y(iced::Center);

    if let Some(artist) = track
        .and_then(verse_core::Track::track_artist)
        .filter(|_| form.artist)
    {
        line = line.push(
            marquee(artist)
                .size(LABEL_FONT_SIZE)
                .style(|theme: &iced::Theme| shade(theme, false)),
        );
    }

    line.width(Length::Fill).into()
}

fn stars<'a>(tracks: Context<'a>, track: Option<&'a verse_core::Track>) -> Element<'a, AppMessage> {
    match tracks.playing.filter(|_| track.is_some()) {
        Some(id) => Rating::new(track.and_then(verse_core::Track::rating))
            .size(STAR_SIZE)
            .spacing(STAR_SPACING)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn name(title: f32, artist: f32) -> NameWidth {
        NameWidth { title, artist }
    }

    fn keeps_stars(name: NameWidth) -> f32 {
        chrome()
            + name.wanted()
            + ROW_GAP
            + Rating::<AppMessage>::width_for(STAR_SIZE, STAR_SPACING)
    }

    fn keeps_artist() -> f32 {
        chrome() + ROW_GAP + SLIVER * 2.0
    }

    #[test]
    fn a_pane_with_room_for_everything_shows_everything() {
        let name = name(150.0, 90.0);
        let form = Form::pick(keeps_stars(name), name);

        assert!(form.stars && form.artist && form.clocks);
    }

    #[test]
    fn the_stars_go_the_moment_they_would_cut_the_name() {
        let name = name(150.0, 90.0);
        let fits = keeps_stars(name);

        assert!(Form::pick(fits, name).stars);
        assert!(
            !Form::pick(fits - 1.0, name).stars,
            "one pixel too narrow for the pair and the stars should already be gone"
        );
    }

    #[test]
    fn the_name_is_never_cut_while_the_stars_remain() {
        let name = name(150.0, 90.0);

        for step in 0..600 {
            let width = step as f32;
            if !Form::pick(width, name).stars {
                continue;
            }

            let room = width
                - chrome()
                - Rating::<AppMessage>::width_for(STAR_SIZE, STAR_SPACING)
                - ROW_GAP;
            assert!(
                room >= name.wanted(),
                "at {width} the stars are drawn but the name is {} short",
                name.wanted() - room
            );
        }
    }

    #[test]
    fn a_short_name_keeps_the_stars_in_a_narrower_pane() {
        let short = name(60.0, 40.0);
        let long = name(300.0, 120.0);
        let pane = keeps_stars(short);

        assert!(keeps_stars(short) < keeps_stars(long));
        assert!(Form::pick(pane, short).stars);
        assert!(
            !Form::pick(pane, long).stars,
            "the same pane cannot hold the stars beside a much longer name"
        );
    }

    #[test]
    fn an_absent_artist_costs_nothing() {
        let alone = name(150.0, 0.0);

        assert!(
            (alone.wanted() - 150.0).abs() < 0.001,
            "a missing artist still charged for its gap"
        );
        assert!(
            !Form::pick(1000.0, alone).artist,
            "a track with no artist drew one anyway"
        );
    }

    #[test]
    fn the_artist_is_cut_rather_than_dropped() {
        for pair in [name(50.0, 30.0), name(400.0, 200.0), name(2000.0, 1500.0)] {
            assert!(
                keeps_artist() < chrome() + pair.wanted(),
                "the artist is dropped though it had room to be cut instead"
            );
            assert!(Form::pick(keeps_artist(), pair).artist);
            assert!(!Form::pick(keeps_artist() - 1.0, pair).artist);
        }
    }

    #[test]
    fn the_clocks_answer_only_to_their_own_row() {
        let long = name(4000.0, 3000.0);
        let pane = chrome() + CLOCKS_FLOOR;

        let form = Form::pick(pane, long);
        assert!(form.clocks);
        assert!(!form.stars, "this pane is far too narrow for the stars");
    }

    #[test]
    fn a_pane_of_no_width_draws_nothing_but_the_rail() {
        let form = Form::pick(0.0, name(150.0, 90.0));
        assert!(!form.stars && !form.artist && !form.clocks);
    }

    #[test]
    fn narrowing_only_ever_drops_things() {
        let name = name(150.0, 90.0);
        let mut last = Form::pick(1000.0, name);

        for step in (0..1000).rev() {
            let form = Form::pick(step as f32, name);

            assert!(!form.stars || last.stars, "the stars came back at {step}");
            assert!(
                !form.artist || last.artist,
                "the artist came back at {step}"
            );
            assert!(
                !form.clocks || last.clocks,
                "the clocks came back at {step}"
            );

            last = form;
        }
    }

    #[test]
    fn a_real_track_keeps_its_artist_into_a_narrow_pane() {
        let name = NameWidth {
            title: marquee::width_of("Mark\u{2019}s Theme", TITLE_FONT_SIZE, TITLE_FONT),
            artist: marquee::width_of(
                "Black Country, New Road",
                LABEL_FONT_SIZE,
                iced::Font::DEFAULT,
            ),
        };

        assert!(
            keeps_artist() < 100.0,
            "the artist is dropped at {}, far wider than the pane it could still fit",
            keeps_artist()
        );
        assert!(Form::pick(keeps_artist(), name).artist);
    }

    #[test]
    fn the_memo_returns_the_same_answer_every_time() {
        let first = NameWidth::measured(Some(1), None);
        let between = NameWidth::measured(Some(2), None);
        let again = NameWidth::measured(Some(1), None);

        let same = |a: NameWidth, b: NameWidth| {
            (a.title - b.title).abs() < 0.001 && (a.artist - b.artist).abs() < 0.001
        };

        assert!(
            same(first, again),
            "the memo changed its mind about a track"
        );
        assert!(same(first, between), "an absent track measured differently");
        assert!(!NameWidth::measured(None, None).has_artist());
    }

    #[test]
    fn a_real_title_is_measured_at_a_plausible_width() {
        let width = marquee::width_of("Black Country, New Road", TITLE_FONT_SIZE, TITLE_FONT);
        assert!(
            width > 50.0 && width < 400.0,
            "a 23-character title measured {width}, which cannot be right"
        );
    }
}
