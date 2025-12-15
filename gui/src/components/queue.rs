use iced::widget::{column, container, scrollable, text};
use iced::{Element, Length, Theme};
use msc_core::Player;

use crate::app::Message;

const MAX_DISPLAY: usize = 100;

pub fn view<'a>(player: &Player) -> Element<'a, Message> {
    let queue = player.queue();
    let library = player.library();
    let current_hash = queue.current_id();

    let mut track_list = column![].spacing(2);

    if let Some(current_id) = current_hash {
        if let Some(track) = library.track_from_id(current_id) {
            track_list = track_list.push(
                container(
                    text(format!(
                        "{} - {}",
                        track.metadata.title_or_default(),
                        track.metadata.artist_or_default()
                    ))
                    .size(14)
                    .style(|theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().primary.weak.text),
                    }),
                )
                .padding(8)
                .width(Length::Fill)
                .style(|theme: &Theme| container::Style {
                    text_color: Some(theme.extended_palette().primary.weak.text),
                    background: Some(theme.extended_palette().primary.weak.color.into()),
                    ..Default::default()
                }),
            );
        }
    }

    let upcoming = queue.upcoming();
    let total_upcoming = upcoming.len();

    for (idx, track_id) in upcoming.iter().enumerate() {
        if idx >= MAX_DISPLAY {
            track_list = track_list.push(
                container(
                    text(format!(
                        "... and {} more tracks",
                        total_upcoming - MAX_DISPLAY
                    ))
                    .size(12)
                    .style(|theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().background.base.text),
                    }),
                )
                .padding(8)
                .width(Length::Fill)
                .style(|theme: &Theme| container::Style {
                    text_color: Some(theme.extended_palette().background.base.text),
                    ..Default::default()
                }),
            );
            break;
        }

        if let Some(track) = library.track_from_id(*track_id) {
            track_list = track_list.push(
                container(
                    text(format!(
                        "{} - {}",
                        track.metadata.title_or_default(),
                        track.metadata.artist_or_default()
                    ))
                    .size(14)
                    .style(|theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().background.base.text),
                    }),
                )
                .padding(8)
                .width(Length::Fill)
                .style(|theme: &Theme| container::Style {
                    text_color: Some(theme.extended_palette().background.base.text),
                    ..Default::default()
                }),
            );
        }
    }

    scrollable(track_list).height(Length::Fill).into()
}
