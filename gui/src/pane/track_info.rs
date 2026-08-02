//! The track info pane: what the track being heard actually is.
//!
//! Two blocks separated by a hairline rule, because the questions they answer
//! are different in kind. The first is the track as a listener names it — title,
//! artist, album and genre — and the second is the file as a decoder sees it:
//! how long, how many channels, at what rate and depth. Reading one does not
//! help you read the other, so they are not one list.
//!
//! The block is left-aligned but centered as a whole. Centering each line
//! individually gives every row a different left edge, so the eye has to find
//! the start of each one; sharing an edge means a column to read down. Centering
//! the block itself is what keeps that from looking marooned in a pane much
//! larger than the text, which is the usual case for a sidebar. The pane is a
//! `responsive` so it can drop the label column on a genuinely narrow pane
//! rather than letting the values wrap into it — see `Density`.
//!
//! The second block is a label/value grid rather than bullet-joined text. Two
//! columns is what makes left alignment worth having: the values line up under
//! each other and the labels say what each one is, where "44.1 kHz • 16 bit •
//! 1006 kbps" asks the reader to know which number is which. Labels are
//! [`crate::styles::LABEL_FONT_SIZE`] and dim, matching the captions in the
//! queue and collections panes.
//!
//! The label column is sized here rather than measured, because a `row!` of two
//! `text` widgets cannot align across separate rows without a shared width.
//! `LABEL_WIDTH` fits the longest label at that size with slack; a label added
//! later that overruns it will clip rather than push the values out of line,
//! which is what `the_label_column_fits_every_label` guards. `WIDE_ENOUGH` is
//! the width below which that column costs more than it explains, and the
//! details fall back to one bullet-joined line.
//!
//! The rows are built from one table of label/reading pairs rather than a
//! statement each, so the two densities cannot drift apart in what they report:
//! `readings` is the single list both read, and the compact path is the same
//! data joined instead of laid out. Duration is separate from that table only
//! because it is the one field a playable file always has, so it needs no
//! `Option` and no filtering.
//!
//! Nothing here allocates per frame beyond the strings iced needs to draw:
//! `joined` borrows its parts and allocates once at the end, the static labels
//! are `&'static str` throughout, and the readings are formatted straight into
//! the `Cow` the text widget takes.
//!
//! Every field a track carries is optional except the duration, and the pane
//! shows only what is actually there rather than a column of dashes. A tag that
//! is missing is not information, so the row it would have occupied closes up
//! instead: `joined` drops the empty parts before deciding whether there is a
//! line at all, which is what keeps a sparsely tagged file from drawing three
//! separators around nothing. Legacy showed `-` for absent values and had to
//! filter them back out again by string comparison downstream; deciding once, up
//! front, is why nothing here compares against a placeholder.
//!
//! The title falls back to the file stem rather than to a dash, since a file
//! with no tags at all is common and its name is very often the only thing
//! anyone knows about it. Dropping to `-` would throw away the one identifier
//! present.
//!
//! `clock` rounds where the timeline scrubber truncates. A scrubber that
//! rounded would show a second that has not elapsed, but this is a static
//! property of the file, where rounding is what agrees with every other tool
//! that reports a duration. That disagreement is deliberate and is why the
//! several `clock` functions across the panes are not shared — see
//! [`crate::pane`].
//!
//! Nothing playing draws nothing at all: a pane that says "no track" is louder
//! than an empty one and says the same thing, given the transport controls are
//! elsewhere.
//!
//! # Color
//!
//! [`crate::pane::settings::Accent`] tints the title, and only the title. The
//! block below it is a table of readings — a sample rate is not more or less the
//! record's for being colored — and the labels beside those readings are already
//! dim so the values can be found; accenting either would take the one contrast
//! the block has and spend it on decoration. The title is the line that names
//! the record, so it is the line that can wear the record's color.
//!
//! Tinting resolves against the theme's primary rather than against the title's
//! own text color, for the reason [`crate::pane::timeline`] gives: an accent
//! keeps the reference's lightness and takes only the hue, and text lightness is
//! near white on a dark theme, where a hue reads as no tint at all.

use std::borrow::Cow;

use iced::alignment::Vertical;
use iced::font::Weight;
use iced::widget::{Space, column, container, responsive, row, text};
use iced::{Element, Font, Length};

use verse_core::Track;

use crate::app::Message;
use crate::browsing::Context;
use crate::pane::settings::{Accent, TrackInfo as Settings};
use crate::styles::{self, LABEL_FONT_SIZE, PAD};

const TITLE_SIZE: f32 = 18.0;
const ARTIST_SIZE: f32 = 14.0;
const DETAIL_SIZE: f32 = 12.0;

const LINE_GAP: f32 = 3.0;
const ROW_GAP: f32 = 5.0;
const BLOCK_GAP: f32 = 12.0;

const LABEL_WIDTH: f32 = 58.0;
const LABEL_GAP: f32 = 10.0;

const WIDE_ENOUGH: f32 = 190.0;

const SEPARATOR: &str = " \u{2022} ";
const NO_DURATION: &str = "\u{2014}";
const UNTITLED: &str = "Unknown";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Density {
    Labeled,
    Compact,
}

impl Density {
    fn for_width(width: f32) -> Self {
        if width >= WIDE_ENOUGH {
            Self::Labeled
        } else {
            Self::Compact
        }
    }
}

pub fn view(
    tracks: Context<'_>,
    settings: Settings,
    cover: Option<[u8; 3]>,
) -> Element<'_, Message> {
    let Some(track) = tracks.playing.and_then(|id| tracks.library.track(id)) else {
        return Space::new().width(Length::Fill).height(Length::Fill).into();
    };

    responsive(move |size| {
        let density = Density::for_width(size.width - PAD * 4.0);

        container(
            column![
                identity(track, settings.accent, cover),
                rule(),
                details(track, density)
            ]
            .spacing(BLOCK_GAP)
            .width(Length::Shrink),
        )
        .padding(PAD * 2.0)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    })
    .into()
}

fn identity(track: &Track, accent: Accent, cover: Option<[u8; 3]>) -> Element<'_, Message> {
    let mut lines = column![
        text(title(track))
            .size(TITLE_SIZE)
            .font(Font {
                weight: Weight::Bold,
                ..Font::DEFAULT
            })
            .style(move |theme: &iced::Theme| text::Style {
                color: Some(styles::accent_heading(theme, accent, cover)),
            })
            .wrapping(text::Wrapping::None),
    ]
    .spacing(LINE_GAP);

    if let Some(artist) = track.track_artist() {
        lines = lines.push(
            text(artist)
                .size(ARTIST_SIZE)
                .wrapping(text::Wrapping::None),
        );
    }

    if let Some(release) = joined([track.album(), track.genre()]) {
        lines = lines.push(value(release));
    }

    lines.into()
}

fn details(track: &Track, density: Density) -> Element<'_, Message> {
    let duration = clock(track.duration());
    let readings = readings(track);

    if density == Density::Compact {
        let shape = match readings[0].1.as_deref() {
            Some(channels) => Cow::Owned(format!("{duration}{SEPARATOR}{channels}")),
            None => duration,
        };

        let mut lines = column![value(shape)].spacing(LINE_GAP);
        if let Some(quality) = joined(readings[1..].iter().map(|(_, reading)| reading.as_deref())) {
            lines = lines.push(value(quality));
        }
        return lines.into();
    }

    let mut rows = column![labeled("Duration", duration)].spacing(ROW_GAP);
    for (label, reading) in readings {
        if let Some(reading) = reading {
            rows = rows.push(labeled(label, reading));
        }
    }

    rows.into()
}

fn readings(track: &Track) -> [(&'static str, Option<Cow<'static, str>>); 4] {
    [
        ("Channels", channels(track.channels())),
        ("Rate", sample_rate(track.sample_rate())),
        (
            "Depth",
            track
                .bit_depth()
                .map(|bits| Cow::Owned(format!("{bits} bit"))),
        ),
        (
            "Bitrate",
            track
                .bit_rate()
                .map(|rate| Cow::Owned(format!("{rate} kbps"))),
        ),
    ]
}

fn labeled<'a>(label: &'static str, reading: Cow<'a, str>) -> Element<'a, Message> {
    row![
        text(label)
            .size(LABEL_FONT_SIZE)
            .style(styles::faint_text)
            .wrapping(text::Wrapping::None)
            .width(Length::Fixed(LABEL_WIDTH)),
        value(reading),
    ]
    .spacing(LABEL_GAP)
    .align_y(Vertical::Center)
    .into()
}

fn value(reading: Cow<'_, str>) -> Element<'_, Message> {
    text(reading)
        .size(DETAIL_SIZE)
        .style(styles::dim_text)
        .wrapping(text::Wrapping::None)
        .into()
}

fn rule<'a>() -> Element<'a, Message> {
    container(Space::new().height(1.0))
        .width(Length::Fill)
        .style(styles::pref_rule_style)
        .into()
}

fn title(track: &Track) -> Cow<'_, str> {
    if let Some(title) = track.title() {
        return Cow::Borrowed(title);
    }

    track
        .path()
        .file_stem()
        .map_or(Cow::Borrowed(UNTITLED), |stem| stem.to_string_lossy())
}

fn joined<'a>(parts: impl IntoIterator<Item = Option<&'a str>>) -> Option<Cow<'static, str>> {
    let mut joined: Option<String> = None;

    for part in parts.into_iter().flatten() {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match &mut joined {
            Some(text) => {
                text.push_str(SEPARATOR);
                text.push_str(part);
            }
            None => joined = Some(part.to_owned()),
        }
    }

    joined.map(Cow::Owned)
}

fn sample_rate(rate: Option<u32>) -> Option<Cow<'static, str>> {
    let rate = rate?;
    Some(Cow::Owned(if rate >= 1000 {
        format!("{:.1} kHz", rate as f32 / 1000.0)
    } else {
        format!("{rate} Hz")
    }))
}

fn channels(channels: Option<u8>) -> Option<Cow<'static, str>> {
    Some(match channels? {
        1 => Cow::Borrowed("Mono"),
        2 => Cow::Borrowed("Stereo"),
        count => Cow::Owned(format!("{count} channels")),
    })
}

fn clock(seconds: f32) -> Cow<'static, str> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Cow::Borrowed(NO_DURATION);
    }
    let total = seconds.round() as u64;
    Cow::Owned(format!("{}:{:02}", total / 60, total % 60))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LABELS: [&str; 5] = ["Duration", "Channels", "Rate", "Depth", "Bitrate"];

    fn widest(label: &str) -> f32 {
        label.chars().count() as f32 * LABEL_FONT_SIZE * 0.62
    }

    #[test]
    fn the_label_column_fits_every_label() {
        for label in LABELS {
            assert!(
                LABEL_WIDTH >= widest(label),
                "{label:?} wants up to {:.1}px but the column is {LABEL_WIDTH}px, \
                 so it would clip or push the values out of line",
                widest(label)
            );
        }
    }

    #[test]
    fn a_wide_pane_gets_the_label_column() {
        assert_eq!(Density::for_width(400.0), Density::Labeled);
        assert_eq!(Density::for_width(WIDE_ENOUGH), Density::Labeled);
    }

    #[test]
    fn a_narrow_pane_drops_to_one_line() {
        assert_eq!(Density::for_width(WIDE_ENOUGH - 1.0), Density::Compact);
        assert_eq!(Density::for_width(0.0), Density::Compact);
    }

    #[test]
    fn a_nonsense_width_still_picks_a_density() {
        assert_eq!(Density::for_width(f32::NAN), Density::Compact);
        assert_eq!(Density::for_width(-100.0), Density::Compact);
    }

    #[test]
    fn the_threshold_leaves_room_for_a_value() {
        let spent = LABEL_WIDTH + LABEL_GAP + PAD * 4.0;
        assert!(
            WIDE_ENOUGH - spent >= 60.0,
            "at the threshold the values get {:.1}px, too little for \"1006 kbps\"",
            WIDE_ENOUGH - spent
        );
    }

    #[test]
    fn a_duration_reads_as_minutes_and_seconds() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(9.0), "0:09");
        assert_eq!(clock(75.0), "1:15");
        assert_eq!(clock(3600.0), "60:00");
    }

    #[test]
    fn a_duration_rounds_to_the_nearest_second() {
        assert_eq!(clock(74.6), "1:15");
        assert_eq!(clock(74.4), "1:14");
    }

    #[test]
    fn a_nonsense_duration_reads_as_a_dash() {
        assert_eq!(clock(f32::NAN), "\u{2014}");
        assert_eq!(clock(-1.0), "\u{2014}");
        assert_eq!(clock(f32::INFINITY), "\u{2014}");
    }

    #[test]
    fn a_rate_over_a_kilohertz_reads_in_kilohertz() {
        assert_eq!(sample_rate(Some(44_100)).as_deref(), Some("44.1 kHz"));
        assert_eq!(sample_rate(Some(48_000)).as_deref(), Some("48.0 kHz"));
        assert_eq!(sample_rate(Some(800)).as_deref(), Some("800 Hz"));
    }

    #[test]
    fn an_absent_rate_is_absent_rather_than_a_dash() {
        assert_eq!(sample_rate(None), None);
        assert_eq!(channels(None), None);
    }

    #[test]
    fn the_common_channel_layouts_are_named() {
        assert_eq!(channels(Some(1)).as_deref(), Some("Mono"));
        assert_eq!(channels(Some(2)).as_deref(), Some("Stereo"));
        assert_eq!(channels(Some(6)).as_deref(), Some("6 channels"));
    }

    #[test]
    fn joining_keeps_only_what_is_there() {
        assert_eq!(
            joined([Some("Album"), None, Some("Genre")]).as_deref(),
            Some("Album \u{2022} Genre")
        );
    }

    #[test]
    fn joining_nothing_is_nothing_rather_than_an_empty_line() {
        assert_eq!(joined([None, None]), None);
    }

    #[test]
    fn a_blank_tag_is_treated_as_absent() {
        assert_eq!(
            joined([Some("  "), Some("Genre")]).as_deref(),
            Some("Genre")
        );
        assert_eq!(joined([Some(""), Some("")]), None);
    }

    #[test]
    fn a_single_part_carries_no_separator() {
        let joined = joined([Some("Album"), None]).expect("one part is still a line");
        assert!(
            !joined.contains(SEPARATOR),
            "{joined:?} has a dangling separator"
        );
    }
}
