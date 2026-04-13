use iced::font::Weight;
use iced::widget::{column, container, mouse_area, rule, scrollable, text};
use iced::{Element, Font, Length, Theme};
use verse_core::Player;

use crate::app::Message;
use crate::art_cache::ArtCache;
use crate::components::context_menu::{MenuElement, context_menu};
use crate::pane_view::{PaneView, ViewContext};
use crate::panes::ControlsMessage;

const MAX_DISPLAY: usize = 100;

#[derive(Debug, Clone)]
pub struct QueuePane;

impl QueuePane {
    pub fn new() -> Self {
        Self
    }
}

impl PaneView for QueuePane {
    fn update(&mut self, _player: &Player, _art: &mut ArtCache) {}

    fn view<'a>(&'a self, ctx: ViewContext<'a>) -> Element<'a, Message> {
        let player = ctx.player;
        let hovered_track = ctx.hovered_track;

        let queue = player.queue();
        let current_id = queue.current_id();

        let mut track_list = column![].spacing(0);

        if let Some(current_id) = current_id {
            if let Ok(Some(track)) = player.query_track_from_id(current_id) {
                let is_hovered = hovered_track.as_ref() == Some(&current_id);

                let track_inner = container(
                    column![
                        text(track.title().unwrap_or("-").to_string())
                            .size(15)
                            .font(Font {
                                weight: Weight::Bold,
                                ..Default::default()
                            })
                            .style(|theme: &Theme| text::Style {
                                color: Some(theme.extended_palette().background.base.text),
                            }),
                        text(track.track_artist().unwrap_or("-").to_string())
                            .size(13)
                            .style(|theme: &Theme| text::Style {
                                color: Some(theme.extended_palette().background.base.text),
                            }),
                    ]
                    .spacing(2),
                )
                .padding(12)
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
                    mouse_area(track_inner).on_move(move |_| Message::TrackHovered(current_id));

                track_list = track_list.push(context_menu(
                    track_content,
                    vec![
                        MenuElement::button(
                            "Shuffle Queue",
                            Message::Controls(ControlsMessage::ShuffleQueue),
                        ),
                        MenuElement::Separator,
                        MenuElement::button("Clear Queue", Message::ClearQueue),
                    ],
                ));
                track_list = track_list.push(container(rule::horizontal(1)).padding([4, 0]));
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
                    .width(Length::Fill),
                );
                break;
            }

            if let Ok(Some(track)) = player.query_track_from_id(*track_id) {
                let is_hovered = hovered_track.as_ref() == Some(track_id);

                let track_inner = container(
                    column![
                        text(track.title().unwrap_or("-").to_string())
                            .size(15)
                            .font(Font {
                                weight: Weight::Bold,
                                ..Default::default()
                            })
                            .style(|theme: &Theme| text::Style {
                                color: Some(theme.extended_palette().background.base.text),
                            }),
                        text(track.track_artist().unwrap_or("-").to_string())
                            .size(13)
                            .style(|theme: &Theme| text::Style {
                                color: Some(theme.extended_palette().background.base.text),
                            }),
                    ]
                    .spacing(2),
                )
                .padding(12)
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
                    mouse_area(track_inner).on_move(move |_| Message::TrackHovered(*track_id));

                let mut items = vec![];
                if idx > 0 {
                    items.push(MenuElement::button(
                        "Move to Top",
                        Message::MoveToQueueFront(idx),
                    ));
                }
                items.push(MenuElement::button("Remove", Message::RemoveFromQueue(idx)));
                items.push(MenuElement::Separator);
                items.push(MenuElement::button(
                    "Shuffle Queue",
                    Message::Controls(ControlsMessage::ShuffleQueue),
                ));
                items.push(MenuElement::Separator);
                items.push(MenuElement::button("Clear Queue", Message::ClearQueue));

                track_list = track_list.push(context_menu(track_content, items));
            }
        }

        if current_id.is_none() && upcoming.is_empty() {
            return container(
                text("Queue is empty")
                    .size(14)
                    .style(|theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().background.strong.text),
                    }),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        }

        mouse_area(scrollable(track_list).height(Length::Fill).direction(
            scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(0).scroller_width(0),
            ),
        ))
        .on_exit(Message::TrackUnhovered)
        .into()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
}
