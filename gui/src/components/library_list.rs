use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};
use verse_core::{Library, Player};

use crate::app::Message;
use crate::components::format_time;
use crate::styles::{self, LABEL_FONT_SIZE, PAD, ROW_FONT_SIZE};

pub fn view<'a>(library: &'a Library, player: &Player) -> Element<'a, Message> {
    if library.tracks().is_empty() {
        return container(
            column![
                text("No music yet").size(16),
                text("Choose a folder to scan for audio files.").size(ROW_FONT_SIZE),
                button(text("Select Folder").size(ROW_FONT_SIZE))
                    .on_press(Message::SelectFolder)
                    .padding(PAD * 2.0),
            ]
            .spacing(PAD * 2.0)
            .align_x(iced::Alignment::Center),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into();
    }

    let current = player.queue().current();

    let mut rows = column![].spacing(1).padding(PAD);
    for track in library.tracks() {
        let Some(id) = track.id() else { continue };

        let title = track.title().unwrap_or_else(|| {
            track
                .path()
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
        });

        let mut label = row![
            text(title)
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

        rows = rows.push(
            row![
                button(label)
                    .on_press(Message::PlayTrack(id))
                    .width(Length::Fill)
                    .padding([PAD, PAD * 2.0])
                    .style(styles::row_style(current == Some(id))),
                button(text("+").size(ROW_FONT_SIZE))
                    .on_press(Message::EnqueueTrack(id))
                    .padding([PAD, PAD * 1.5])
                    .style(styles::icon_button_style),
            ]
            .align_y(iced::Alignment::Center),
        );
    }

    scrollable(rows).height(Length::Fill).into()
}
