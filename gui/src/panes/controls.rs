use iced::alignment::Vertical;
use iced::font::Weight;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{column, container, responsive, row, svg, text, tooltip};
use iced::{Element, Font, Length, Theme};
use msc_core::Player;

use crate::app::Message;
use crate::art_cache::ArtCache;
use crate::formatters::format_duration;
use crate::pane_view::{PaneView, ViewContext};
use crate::styles::{TOOLTIP_DELAY, svg_style};
use crate::widgets::canvas_button::canvas_button;
use crate::widgets::hover_slider::hover_slider;

#[derive(Debug, Clone)]
pub struct ControlsPane;

impl ControlsPane {
    pub fn new() -> Self {
        Self
    }
}

impl PaneView for ControlsPane {
    fn update(&mut self, _player: &Player, _art: &mut ArtCache) {}

    fn view<'a>(&'a self, ctx: ViewContext<'a>) -> Element<'a, Message> {
        let player = ctx.player;
        let volume = ctx.volume;
        let seeking_position = ctx.seeking_position;

        let is_playing = player.is_playing();
        let current_track = player.clone_current_track();
        let actual_position = player.position() as f32;
        let position = seeking_position.unwrap_or(actual_position);

        let (title, artist, duration) = if let Some(track) = current_track {
            (
                track.title().unwrap_or("-").to_string(),
                track.track_artist().unwrap_or("-").to_string(),
                track.duration(),
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
                .height(22)
                .style(svg_style),
            )
            .width(22)
            .height(22)
            .on_press(Message::Controls(ControlsMessage::Previous)),
            container(text("Previous Track").size(12))
                .padding(6)
                .style(container::rounded_box),
            tooltip::Position::Top,
        )
        .gap(8)
        .delay(TOOLTIP_DELAY)
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
                    .height(28)
                    .style(svg_style),
            )
            .width(28)
            .height(28)
            .on_press(Message::Controls(ControlsMessage::PlayPause)),
            container(text(play_pause_tooltip).size(12))
                .padding(6)
                .style(container::rounded_box),
            tooltip::Position::Top,
        )
        .gap(8)
        .delay(TOOLTIP_DELAY)
        .snap_within_viewport(true);

        let next_button = tooltip(
            canvas_button(
                svg(SvgHandle::from_memory(include_bytes!(
                    "../../../assets/icons/next.svg"
                )))
                .width(22)
                .height(22)
                .style(svg_style),
            )
            .width(22)
            .height(22)
            .on_press(Message::Controls(ControlsMessage::Next)),
            container(text("Next Track").size(12))
                .padding(6)
                .style(container::rounded_box),
            tooltip::Position::Top,
        )
        .gap(8)
        .delay(TOOLTIP_DELAY)
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
                    .height(22)
                    .style(svg_style),
            )
            .width(22)
            .height(22)
            .on_press(Message::Controls(ControlsMessage::ToggleMute)),
            container(text(vol_tooltip).size(12))
                .padding(6)
                .style(container::rounded_box),
            tooltip::Position::Top,
        )
        .gap(8)
        .delay(TOOLTIP_DELAY)
        .snap_within_viewport(true);

        let volume_slider = hover_slider(0.0..=1.0, volume, |v| {
            Message::Controls(ControlsMessage::VolumeChanged(v))
        })
        .step(0.01)
        .width(Length::Fixed(100.0));

        let shuffle_button = tooltip(
            canvas_button(
                svg(SvgHandle::from_memory(include_bytes!(
                    "../../../assets/icons/shuffle.svg"
                )))
                .width(22)
                .height(22)
                .style(svg_style),
            )
            .width(22)
            .height(22)
            .on_press(Message::Controls(ControlsMessage::ShuffleQueue)),
            container(text("Shuffle Queue").size(12))
                .padding(6)
                .style(container::rounded_box),
            tooltip::Position::Top,
        )
        .gap(8)
        .delay(TOOLTIP_DELAY)
        .snap_within_viewport(true);

        let cycle_button = tooltip(
            canvas_button(
                svg(SvgHandle::from_memory(include_bytes!(
                    "../../../assets/icons/cycle.svg"
                )))
                .width(22)
                .height(22)
                .style(svg_style),
            )
            .width(22)
            .height(22)
            .on_press(Message::Controls(ControlsMessage::CycleLoopMode)),
            container(text("Cycle Loop Mode").size(12))
                .padding(6)
                .style(container::rounded_box),
            tooltip::Position::Top,
        )
        .gap(8)
        .delay(TOOLTIP_DELAY)
        .snap_within_viewport(true);

        let time_text = format!(
            "{} / {}",
            format_duration(position),
            format_duration(duration)
        );

        let track_info = responsive(move |size| {
            let available_width = size.width - 100.0;
            let max_chars = (available_width / 7.0) as usize;
            let title_max = max_chars.max(10) / 2;
            let artist_max = max_chars.max(10) / 2;

            let truncated_title = truncate_text(&title, title_max);
            let truncated_artist = truncate_text(&artist, artist_max);

            let timeline_slider = hover_slider(0.0..=duration, position, |v| {
                Message::Controls(ControlsMessage::SeekChanged(v))
            })
            .on_release(Message::Controls(ControlsMessage::SeekReleased))
            .width(Length::Fill);

            container(
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
                    ]
                    .align_y(Vertical::Center),
                    timeline_slider,
                ]
                .spacing(0)
                .width(Length::Fill),
            )
            .center_y(Length::Fill)
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
            track_info,
            container(text("")).width(Length::Fixed(20.0)),
            shuffle_button,
            cycle_button,
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

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
}

// these should prob just exist in the main set of messages
#[derive(Debug, Clone)]
pub enum ControlsMessage {
    PlayPause,
    Previous,
    Next,
    VolumeChanged(f32),
    ToggleMute,
    SeekChanged(f32),
    SeekReleased,
    ShuffleQueue,
    CycleLoopMode,
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
