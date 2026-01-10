use iced::alignment::Horizontal;
use iced::widget::{button, column, container, mouse_area, row, scrollable, text};
use iced::{Element, Length, Theme};
use msc_core::{Player, Track};

use crate::app::Message;
use crate::components::context_menu::{MenuElement, context_menu};
use crate::formatters;

pub fn view<'a>(
    _player: &Player,
    hovered_track: &Option<i64>,
    cached_tracks: Vec<Track>,
) -> Element<'a, Message> {
    let tracks: Vec<_> = cached_tracks.iter().collect();

    if tracks.is_empty() {
        return container(
            column![
                text("No library")
                    .size(18)
                    .style(|theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().background.base.text),
                    }),
                button(text("Set directory").size(14))
                    .on_press(Message::SetLibrary)
                    .padding(10),
            ]
            .spacing(20)
            .align_x(Horizontal::Center),
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|theme: &Theme| container::Style {
            text_color: Some(theme.extended_palette().background.base.text),
            ..Default::default()
        })
        .into();
    }

    let header = container(
        row![
            container(text("Title").size(12).style(|theme: &Theme| text::Style {
                color: Some(theme.extended_palette().background.strong.text),
            }))
            .width(Length::FillPortion(3)),
            container(text("Artist").size(12).style(|theme: &Theme| text::Style {
                color: Some(theme.extended_palette().background.strong.text),
            }))
            .width(Length::FillPortion(2)),
            container(text("Album").size(12).style(|theme: &Theme| text::Style {
                color: Some(theme.extended_palette().background.strong.text),
            }))
            .width(Length::FillPortion(2)),
            container(
                text("Duration")
                    .size(12)
                    .style(|theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().background.strong.text),
                    })
            )
            .width(Length::Fixed(80.0)),
        ]
        .spacing(10),
    )
    .padding(10)
    .width(Length::Fill)
    .style(|theme: &Theme| container::Style {
        text_color: Some(theme.extended_palette().background.strong.text),
        background: Some(theme.extended_palette().background.strong.color.into()),
        ..Default::default()
    });

    let mut track_list = column![].spacing(0);

    for track in tracks.iter() {
        if let Some(track_id) = track.id() {
            let duration_text = formatters::format_duration(track.duration());
            let is_hovered = hovered_track.as_ref() == Some(&track_id);

            let track_inner = container(
                row![
                    container(text(track.title_or_default().to_string()).size(12))
                        .width(Length::FillPortion(3)),
                    container(text(track.track_artist_or_default().to_string()).size(12))
                        .width(Length::FillPortion(2)),
                    container(text(track.album_or_default().to_string()).size(12))
                        .width(Length::FillPortion(2)),
                    container(text(duration_text).size(12)).width(Length::Fixed(80.0)),
                ]
                .spacing(10),
            )
            .padding(10)
            .width(Length::Fill)
            .style(move |theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    text_color: Some(palette.background.base.text),
                    background: if is_hovered {
                        Some(palette.primary.weak.color.into())
                    } else {
                        Some(palette.background.base.color.into())
                    },
                    ..Default::default()
                }
            });

            let track_content =
                mouse_area(track_inner).on_move(move |_| Message::TrackHovered(track_id));

            // add dropdown to make new playlist or add to existing
            let track_row = context_menu(
                track_content,
                vec![
                    MenuElement::button("Add to playlist", Message::Tick),
                    MenuElement::Separator,
                    MenuElement::button("Play", Message::PlayTrack(track_id)),
                    MenuElement::button("Queue next", Message::QueueFront(track_id)),
                    MenuElement::button("Queue", Message::QueueBack(track_id)),
                    MenuElement::Separator,
                    MenuElement::button("Queue library", Message::QueueLibrary),
                ],
                Length::Fixed(130.),
            );

            track_list = track_list.push(track_row);
        }
    }

    mouse_area(
        column![
            header,
            scrollable(track_list)
                .height(Length::Fill)
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(0).scroller_width(0),
                ))
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .on_exit(Message::TrackUnhovered)
    .into()
}
