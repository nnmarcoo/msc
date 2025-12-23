use iced::alignment::Vertical;
use iced::font::Weight;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{column, container, responsive, row, svg, text, tooltip};
use iced::{Element, Font, Length, Theme};
use msc_core::Player;

use crate::formatters;
use crate::widgets::canvas_button::canvas_button;
use crate::widgets::hover_slider::hover_slider;

#[derive(Debug, Clone)]
pub enum Message {
    PlayPause,
    Previous,
    Next,
    VolumeChanged(f32),
    ToggleMute,
    SeekChanged(f32),
    SeekReleased,
}

pub fn view<'a>(
    player: &Player,
    volume: f32,
    seeking_position: Option<f32>,
) -> Element<'a, Message> {
    let is_playing = player.is_playing();
    let current_track = player.clone_current_track();
    let actual_position = player.position() as f32;
    let position = seeking_position.unwrap_or(actual_position);

    let (title, artist, duration) = if let Some(track) = current_track {
        (
            track.metadata.title_or_default().to_string(),
            track.metadata.artist_or_default().to_string(),
            track.metadata.duration,
        )
    } else {
        ("-".to_string(), "-".to_string(), 0.0)
    };

    let prev_button = tooltip(
        canvas_button(
            svg(SvgHandle::from_memory(include_bytes!(
                "../../../assets/icons/previous.svg"
            )))
            .width(22)
            .height(22),
        )
        .width(22)
        .height(22)
        .on_press(Message::Previous),
        container(text("Previous track").size(12))
            .padding(6)
            .style(container::rounded_box),
        tooltip::Position::Top,
    )
    .gap(8)
    .snap_within_viewport(true);

    let play_pause_icon: &[u8] = if is_playing {
        include_bytes!("../../../assets/icons/pause.svg")
    } else {
        include_bytes!("../../../assets/icons/play.svg")
    };
    let play_pause_tooltip = if is_playing { "Pause" } else { "Play" };
    let play_pause_button = tooltip(
        canvas_button(
            svg(SvgHandle::from_memory(play_pause_icon))
                .width(28)
                .height(28),
        )
        .width(28)
        .height(28)
        .on_press(Message::PlayPause),
        container(text(play_pause_tooltip).size(12))
            .padding(6)
            .style(container::rounded_box),
        tooltip::Position::Top,
    )
    .gap(8)
    .snap_within_viewport(true);

    let next_button = tooltip(
        canvas_button(
            svg(SvgHandle::from_memory(include_bytes!(
                "../../../assets/icons/next.svg"
            )))
            .width(22)
            .height(22),
        )
        .width(22)
        .height(22)
        .on_press(Message::Next),
        container(text("Next track").size(12))
            .padding(6)
            .style(container::rounded_box),
        tooltip::Position::Top,
    )
    .gap(8)
    .snap_within_viewport(true);

    let vol_icon_bytes: &[u8] = if volume > 0. {
        include_bytes!("../../../assets/icons/vol_on.svg")
    } else {
        include_bytes!("../../../assets/icons/vol_off.svg")
    };
    let vol_tooltip = if volume > 0.0 { "Mute" } else { "Unmute" };
    let vol_button = tooltip(
        canvas_button(
            svg(SvgHandle::from_memory(vol_icon_bytes))
                .width(22)
                .height(22),
        )
        .width(22)
        .height(22)
        .on_press(Message::ToggleMute),
        container(text(vol_tooltip).size(12))
            .padding(6)
            .style(container::rounded_box),
        tooltip::Position::Top,
    )
    .gap(8)
    .snap_within_viewport(true);

    let volume_slider = hover_slider(0.0..=1.0, volume, Message::VolumeChanged)
        .step(0.01)
        .width(Length::Fixed(100.0));

    let time_text = format!(
        "{} / {}",
        formatters::format_duration(position),
        formatters::format_duration(duration)
    );

    let track_info = responsive(move |size| {
        let available_width = size.width - 100.0;
        let max_chars = (available_width / 7.0) as usize;
        let title_max = max_chars.max(10) / 2;
        let artist_max = max_chars.max(10) / 2;

        let truncated_title = truncate_text(&title, title_max);
        let truncated_artist = truncate_text(&artist, artist_max);

        let timeline_slider = hover_slider(0.0..=duration, position, Message::SeekChanged)
            .on_release(Message::SeekReleased)
            .width(Length::Fill);

        column![
            row![
                text(truncated_title)
                    .size(14)
                    .font(Font {
                        weight: Weight::Bold,
                        ..Default::default()
                    })
                    .style(|theme: &Theme| {
                        text::Style {
                            color: Some(theme.extended_palette().background.base.text),
                        }
                    }),
                text(" ").size(14),
                text(truncated_artist).size(14).style(|theme: &Theme| {
                    text::Style {
                        color: Some(theme.extended_palette().background.base.text),
                    }
                }),
                container(text(time_text.clone()).size(14).style(|theme: &Theme| {
                    text::Style {
                        color: Some(theme.extended_palette().background.base.text),
                    }
                }))
                .width(Length::Fill)
                .align_right(Length::Fill),
            ],
            timeline_slider,
        ]
        .spacing(0)
        .width(Length::Fill)
        .into()
    });

    let controls_row = row![
        prev_button,
        play_pause_button,
        next_button,
        container(text("")).width(Length::Fixed(20.0)),
        vol_button,
        volume_slider,
        container(text("")).width(Length::Fixed(20.0)),
        container(track_info).center_y(Length::Fill),
    ]
    .spacing(10)
    .padding(15)
    .align_y(Vertical::Center);

    container(controls_row)
        .width(Length::Fill)
        .height(Length::Fixed(80.0))
        .center_y(Length::Fill)
        .into()
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
