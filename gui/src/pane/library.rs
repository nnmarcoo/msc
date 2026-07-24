//! Library pane: the full track list, with a per-pane search filter.

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};
use verse_core::{Library, Player, Track};

use crate::app::Message as AppMessage;
use crate::components::format_time;
use crate::layout::PaneId;
use crate::pane::PaneMessage;
use crate::styles::{self, LABEL_FONT_SIZE, PAD, ROW_FONT_SIZE};

#[derive(Debug, Default)]
pub struct State {
    pub search: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
}

pub fn update(state: &mut State, message: Message) {
    match message {
        Message::SearchChanged(query) => state.search = query,
    }
}

pub fn view<'a>(
    id: PaneId,
    state: &'a State,
    library: &'a Library,
    player: &Player,
) -> Element<'a, AppMessage> {
    if library.tracks().is_empty() {
        return empty_state();
    }

    let current = player.queue().current();
    let query = state.search.to_lowercase();

    let mut rows = column![].spacing(1);
    for track in library.tracks() {
        let Some(track_id) = track.id() else { continue };
        if !matches_query(track, &query) {
            continue;
        }
        rows = rows.push(track_row(track, track_id, current == Some(track_id)));
    }

    column![
        container(
            text_input("Search…", &state.search)
                .on_input(move |query| {
                    AppMessage::Pane(id, PaneMessage::Library(Message::SearchChanged(query)))
                })
                .size(ROW_FONT_SIZE)
                .padding(PAD)
        )
        .padding(PAD),
        scrollable(rows.padding([0, PAD])).height(Length::Fill),
    ]
    .into()
}

fn matches_query(track: &Track, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let contains =
        |field: Option<&str>| field.is_some_and(|value| value.to_lowercase().contains(query));

    contains(track.title())
        || contains(track.track_artist())
        || contains(track.album())
        || contains(track.album_artist())
}

fn track_row<'a>(track: &'a Track, track_id: i64, is_current: bool) -> Element<'a, AppMessage> {
    let mut label = row![
        text(display_title(track))
            .size(ROW_FONT_SIZE)
            .width(Length::FillPortion(3)),
        text(track.track_artist().unwrap_or("—"))
            .size(ROW_FONT_SIZE)
            .width(Length::FillPortion(2)),
        text(format_time(f64::from(track.duration()))).size(LABEL_FONT_SIZE),
    ]
    .spacing(PAD * 2.0)
    .align_y(iced::Alignment::Center);

    if track.missing() {
        label = label.push(text("missing").size(LABEL_FONT_SIZE));
    }

    row![
        button(label)
            .on_press(AppMessage::PlayTrack(track_id))
            .width(Length::Fill)
            .padding([PAD, PAD * 2.0])
            .style(styles::row_style(is_current)),
        button(text("+").size(ROW_FONT_SIZE))
            .on_press(AppMessage::EnqueueTrack(track_id))
            .padding([PAD, PAD * 1.5])
            .style(styles::icon_button_style),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}

fn display_title(track: &Track) -> &str {
    track.title().unwrap_or_else(|| {
        track
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Unknown")
    })
}

fn empty_state<'a>() -> Element<'a, AppMessage> {
    container(
        column![
            text("No music yet").size(16),
            text("Choose a folder to scan for audio files.").size(ROW_FONT_SIZE),
            button(text("Select Folder").size(ROW_FONT_SIZE))
                .on_press(AppMessage::SelectFolder)
                .padding(PAD * 2.0),
        ]
        .spacing(PAD * 2.0)
        .align_x(iced::Alignment::Center),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}
