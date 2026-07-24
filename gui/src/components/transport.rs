//! The one fixed region of the shell: playback controls, scrubber, and the
//! layout preset switcher.

use iced::widget::{button, column, container, row, slider, text};
use iced::{Element, Length};
use verse_core::{Library, LoopMode, Player, Track};

use crate::app::Message;
use crate::components::format_time;
use crate::layout::Layout;
use crate::styles::{self, BAR_HEIGHT, LABEL_FONT_SIZE, PAD, ROW_FONT_SIZE};

pub struct Context<'a> {
    pub library: &'a Library,
    pub player: &'a Player,
    pub seeking: Option<f32>,
    pub scanning: bool,
    pub status: Option<&'a str>,
    pub edit_mode: bool,
    pub layouts: &'a [Layout],
    pub active_layout: usize,
}

pub fn view<'a>(ctx: Context<'a>) -> Element<'a, Message> {
    let track = ctx.player.current_track(ctx.library);
    let duration = track.map_or(0.0, |t| f64::from(t.duration()));
    let position = ctx.seeking.map_or(ctx.player.position(), f64::from);

    let idle_label = if ctx.scanning {
        "Scanning…"
    } else {
        "Nothing playing"
    };

    let now_playing = column![
        text(track.and_then(Track::title).unwrap_or(idle_label)).size(ROW_FONT_SIZE),
        text(
            ctx.status
                .or_else(|| track.and_then(Track::track_artist))
                .unwrap_or("—")
        )
        .size(LABEL_FONT_SIZE),
    ]
    .spacing(2)
    .width(Length::FillPortion(2));

    let controls = row![
        control("⏮", Message::Previous),
        control(
            if ctx.player.is_playing() { "⏸" } else { "▶" },
            Message::PlayPause
        ),
        control("⏭", Message::Next),
        control("🔀", Message::Shuffle),
        control(loop_label(ctx.player.loop_mode()), Message::CycleLoop),
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

    let volume = slider(0.0..=1.0, ctx.player.volume(), Message::Volume)
        .step(0.01)
        .width(Length::Fixed(80.0));

    container(
        row![
            now_playing,
            controls,
            scrubber,
            volume,
            presets(ctx.layouts, ctx.active_layout),
            actions(ctx.library, ctx.scanning, ctx.edit_mode),
        ]
        .spacing(PAD * 3.0)
        .align_y(iced::Alignment::Center),
    )
    .height(Length::Fixed(BAR_HEIGHT))
    .padding([PAD, PAD * 2.0])
    .style(styles::bar_style)
    .into()
}

fn presets(layouts: &[Layout], active: usize) -> Element<'_, Message> {
    let mut buttons = row![].spacing(2);
    for (index, layout) in layouts.iter().enumerate() {
        buttons = buttons.push(
            button(text(layout.name.as_str()).size(LABEL_FONT_SIZE))
                .on_press(Message::SelectLayout(index))
                .padding([2.0, PAD])
                .style(styles::toggle_style(index == active)),
        );
    }
    buttons.into()
}

fn actions<'a>(library: &Library, scanning: bool, edit_mode: bool) -> Element<'a, Message> {
    let has_root = library.root().is_some();
    let scan_label = if has_root { "Rescan" } else { "Select Folder" };
    let scan_message = if has_root {
        Message::Rescan
    } else {
        Message::SelectFolder
    };

    row![
        button(text(scan_label).size(LABEL_FONT_SIZE))
            .on_press_maybe((!scanning).then_some(scan_message))
            .padding([2.0, PAD])
            .style(styles::icon_button_style),
        button(text("Queue All").size(LABEL_FONT_SIZE))
            .on_press(Message::QueueAll)
            .padding([2.0, PAD])
            .style(styles::icon_button_style),
        button(text("Edit").size(LABEL_FONT_SIZE))
            .on_press(Message::ToggleEditMode)
            .padding([2.0, PAD])
            .style(styles::toggle_style(edit_mode)),
    ]
    .spacing(PAD)
    .into()
}

fn control<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
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
