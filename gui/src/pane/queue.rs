//! Queue pane: what is playing now and what follows.

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};
use verse_core::{Library, Player, Track};

use crate::app::Message as AppMessage;
use crate::layout::PaneId;
use crate::styles::{self, LABEL_FONT_SIZE, PAD, ROW_FONT_SIZE};

#[derive(Debug, Default)]
pub struct State {
    pub show_history: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleHistory,
}

pub fn update(state: &mut State, message: Message) {
    match message {
        Message::ToggleHistory => state.show_history = !state.show_history,
    }
}

pub fn view<'a>(
    _id: PaneId,
    _state: &'a State,
    library: &'a Library,
    player: &Player,
) -> Element<'a, AppMessage> {
    let queue = player.queue();

    let header = row![
        text("Queue").size(ROW_FONT_SIZE).width(Length::Fill),
        button(text("Clear").size(LABEL_FONT_SIZE))
            .on_press(AppMessage::ClearQueue)
            .padding([2.0, PAD])
            .style(styles::icon_button_style),
    ]
    .spacing(PAD)
    .align_y(iced::Alignment::Center);

    let mut rows = column![].spacing(1);

    if let Some(track) = queue.current().and_then(|id| library.track(id)) {
        rows = rows.push(
            container(
                column![
                    text("NOW PLAYING").size(LABEL_FONT_SIZE),
                    text(display_title(track)).size(ROW_FONT_SIZE),
                    text(track.track_artist().unwrap_or("—")).size(LABEL_FONT_SIZE),
                ]
                .spacing(2),
            )
            .padding([PAD, PAD * 2.0])
            .width(Length::Fill),
        );
    }

    for (index, &track_id) in queue.upcoming().iter().enumerate() {
        let Some(track) = library.track(track_id) else {
            continue;
        };

        rows = rows.push(
            row![
                button(
                    column![
                        text(display_title(track)).size(ROW_FONT_SIZE),
                        text(track.track_artist().unwrap_or("—")).size(LABEL_FONT_SIZE),
                    ]
                    .spacing(1)
                )
                .on_press(AppMessage::PlayTrack(track_id))
                .width(Length::Fill)
                .padding([PAD, PAD * 2.0])
                .style(styles::row_style(false)),
                button(text("×").size(ROW_FONT_SIZE))
                    .on_press(AppMessage::RemoveFromQueue(index))
                    .padding([PAD, PAD * 1.5])
                    .style(styles::icon_button_style),
            ]
            .align_y(iced::Alignment::Center),
        );
    }

    if queue.is_empty() {
        rows = rows.push(
            container(text("Nothing queued").size(LABEL_FONT_SIZE))
                .padding(PAD * 2.0)
                .center_x(Length::Fill),
        );
    }

    column![
        container(header).padding([PAD, PAD * 2.0]),
        scrollable(rows).height(Length::Fill),
    ]
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
