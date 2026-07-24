use iced::widget::{button, column, container, row, slider, text};
use iced::{Element, Length};
use verse_core::{Library, LoopMode, Player};

use crate::app::Message;
use crate::components::format_time;
use crate::styles::{self, BAR_HEIGHT, LABEL_FONT_SIZE, PAD, ROW_FONT_SIZE};

pub fn view<'a>(
    library: &'a Library,
    player: &Player,
    seeking: Option<f32>,
    scanning: bool,
    status: Option<&'a str>,
) -> Element<'a, Message> {
    let track = player.current_track(library);
    let duration = track.map_or(0.0, |t| f64::from(t.duration()));
    let position = seeking.map_or(player.position(), f64::from);

    let now_playing = column![
        text(track.and_then(|t| t.title()).unwrap_or(if scanning {
            "Scanning…"
        } else {
            "Nothing playing"
        }))
        .size(ROW_FONT_SIZE),
        text(
            status
                .or_else(|| track.and_then(|t| t.track_artist()))
                .unwrap_or("—")
        )
        .size(LABEL_FONT_SIZE),
    ]
    .spacing(2)
    .width(Length::FillPortion(2));

    let controls = row![
        control("⏮", Message::Previous),
        control(
            if player.is_playing() { "⏸" } else { "▶" },
            Message::PlayPause
        ),
        control("⏭", Message::Next),
        control("🔀", Message::Shuffle),
        control(loop_label(player.loop_mode()), Message::CycleLoop),
    ]
    .spacing(PAD)
    .align_y(iced::Alignment::Center);

    let scrubber = row![
        text(format_time(position)).size(LABEL_FONT_SIZE),
        slider(
            0.0..=duration.max(0.1) as f32,
            position as f32,
            Message::Seek
        )
        .on_release(Message::SeekReleased)
        .width(Length::Fill),
        text(format_time(duration)).size(LABEL_FONT_SIZE),
    ]
    .spacing(PAD)
    .align_y(iced::Alignment::Center)
    .width(Length::FillPortion(3));

    let volume = slider(0.0..=1.0, player.volume(), Message::Volume)
        .step(0.01)
        .width(Length::Fixed(80.0));

    let library_actions = row![
        button(
            text(if library.root().is_some() {
                "Rescan"
            } else {
                "Select Folder"
            })
            .size(LABEL_FONT_SIZE)
        )
        .on_press_maybe((!scanning).then_some(if library.root().is_some() {
            Message::Rescan
        } else {
            Message::SelectFolder
        }))
        .padding([2.0, PAD])
        .style(styles::icon_button_style),
        button(text("Queue All").size(LABEL_FONT_SIZE))
            .on_press(Message::QueueAll)
            .padding([2.0, PAD])
            .style(styles::icon_button_style),
    ]
    .spacing(PAD);

    container(
        row![now_playing, controls, scrubber, volume, library_actions]
            .spacing(PAD * 3.0)
            .align_y(iced::Alignment::Center),
    )
    .height(Length::Fixed(BAR_HEIGHT))
    .padding([PAD, PAD * 2.0])
    .style(styles::bar_style)
    .into()
}

fn control(label: &str, message: Message) -> Element<'_, Message> {
    button(text(label).size(ROW_FONT_SIZE))
        .on_press(message)
        .padding([PAD, PAD * 1.5])
        .style(styles::icon_button_style)
        .into()
}

fn loop_label(mode: LoopMode) -> &'static str {
    match mode {
        LoopMode::None => "↻ off",
        LoopMode::Queue => "↻ all",
        LoopMode::Single => "↻ one",
    }
}
