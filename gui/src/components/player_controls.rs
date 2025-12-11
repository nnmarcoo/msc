use iced::alignment::Vertical;
use iced::widget::image::Handle;
use iced::widget::{Image, button, column, container, row, slider, text};
use iced::{Element, Length};
use msc_core::Player;

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

pub fn view<'a>(player: &Player, volume: f32) -> Element<'a, Message> {
    let is_playing = player.is_playing();
    let current_track = player.clone_current_track();
    let position = player.position() as f32;

    let (title, artist, duration) = if let Some(track) = current_track {
        (
            track.metadata.title_or_default().to_string(),
            track.metadata.artist_or_default().to_string(),
            track.metadata.duration,
        )
    } else {
        ("-".to_string(), "-".to_string(), 0.0)
    };

    let prev_button = button(
        Image::new(Handle::from_bytes(
            include_bytes!("../../../assets/icons/previous.png").as_slice(),
        ))
        .width(22)
        .height(22),
    )
    .padding(8)
    .on_press(Message::Previous);

    let play_pause_icon: &[u8] = if is_playing {
        include_bytes!("../../../assets/icons/pause.png")
    } else {
        include_bytes!("../../../assets/icons/play.png")
    };
    let play_pause_button = button(
        Image::new(Handle::from_bytes(play_pause_icon))
            .width(28)
            .height(28),
    )
    .padding(10)
    .on_press(Message::PlayPause);

    let next_button = button(
        Image::new(Handle::from_bytes(
            include_bytes!("../../../assets/icons/next.png").as_slice(),
        ))
        .width(22)
        .height(22),
    )
    .padding(8)
    .on_press(Message::Next);

    let vol_icon_bytes: &[u8] = if volume > 0.0 {
        include_bytes!("../../../assets/icons/vol_on.png")
    } else {
        include_bytes!("../../../assets/icons/vol_off.png")
    };
    let vol_button = button(
        Image::new(Handle::from_bytes(vol_icon_bytes))
            .width(22)
            .height(22),
    )
    .padding(6)
    .on_press(Message::ToggleMute);

    let volume_slider = slider(0.0..=1.0, volume, Message::VolumeChanged)
        .step(0.01)
        .width(Length::Fixed(100.0));

    let timeline_slider = slider(0.0..=duration, position, Message::SeekChanged)
        .on_release(Message::SeekReleased)
        .width(Length::Fill);

    let time_text = format!(
        "{} / {}",
        format_seconds(position),
        format_seconds(duration)
    );

    let track_info = column![
        row![
            text(title).size(14),
            text(" - ").size(14),
            text(artist).size(14),
        ]
        .spacing(5),
        timeline_slider,
        text(time_text).size(12),
    ]
    .spacing(5)
    .width(Length::Fill);

    let controls_row = row![
        prev_button,
        play_pause_button,
        next_button,
        container(text("")).width(Length::Fixed(20.0)),
        vol_button,
        volume_slider,
        container(text("")).width(Length::Fixed(20.0)),
        track_info,
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

fn format_seconds(seconds: f32) -> String {
    let total_secs = seconds as u32;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", mins, secs)
}
