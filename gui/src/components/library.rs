use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Length, Theme};
use msc_core::Player;

use crate::app::Message;

pub fn view<'a>(player: &Player) -> Element<'a, Message> {
    let library = player.library();

    let tracks = if let Some(track_map) = &library.tracks {
        let mut tracks: Vec<_> = track_map
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        tracks.sort_by(|a, b| {
            a.metadata
                .artist_or_default()
                .cmp(&b.metadata.artist_or_default())
                .then_with(|| {
                    a.metadata
                        .album_or_default()
                        .cmp(&b.metadata.album_or_default())
                })
                .then_with(|| {
                    a.metadata
                        .title_or_default()
                        .cmp(&b.metadata.title_or_default())
                })
        });
        tracks
    } else {
        Vec::new()
    };

    if tracks.is_empty() {
        return container(
            column![
                text("No Library Loaded")
                    .size(18)
                    .style(|theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().background.base.text),
                    }),
                text("Load a library to see your music")
                    .size(14)
                    .style(|theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().background.base.text),
                    }),
            ]
            .spacing(10),
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

    for track in tracks {
        let duration_text = format_seconds(track.metadata.duration);

        let track_row = container(
            row![
                container(text(track.metadata.title_or_default()).size(12).style(
                    |theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().background.base.text),
                    }
                ))
                .width(Length::FillPortion(3)),
                container(text(track.metadata.artist_or_default()).size(12).style(
                    |theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().background.base.text),
                    }
                ))
                .width(Length::FillPortion(2)),
                container(text(track.metadata.album_or_default()).size(12).style(
                    |theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().background.base.text),
                    }
                ))
                .width(Length::FillPortion(2)),
                container(
                    text(duration_text)
                        .size(12)
                        .style(|theme: &Theme| text::Style {
                            color: Some(theme.extended_palette().background.base.text),
                        })
                )
                .width(Length::Fixed(80.0)),
            ]
            .spacing(10),
        )
        .padding(10)
        .width(Length::Fill)
        .style(|theme: &Theme| container::Style {
            text_color: Some(theme.extended_palette().background.base.text),
            background: Some(theme.extended_palette().background.base.color.into()),
            ..Default::default()
        });

        track_list = track_list.push(track_row);
    }

    column![header, scrollable(track_list).height(Length::Fill)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn format_seconds(seconds: f32) -> String {
    let total_secs = seconds as u32;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", mins, secs)
}
