use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, pane_grid, row, text};
use iced::{Border, Element, Length, Theme};
use msc_core::Player;

use crate::elements;
use crate::layout::Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneContent {
    PlayerControls,
    Queue,
    Library,
    Artwork,
    Timeline,
    Empty,
}

impl PaneContent {
    pub fn title(&self) -> &str {
        match self {
            PaneContent::PlayerControls => "Player Controls",
            PaneContent::Queue => "Queue",
            PaneContent::Library => "Library",
            PaneContent::Artwork => "Artwork",
            PaneContent::Timeline => "Timeline",
            PaneContent::Empty => "Empty",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pane {
    pub content: PaneContent,
}

impl Pane {
    pub fn new(content: PaneContent) -> Self {
        Self { content }
    }
    pub fn view(
        &self,
        pane: pane_grid::Pane,
        total_panes: usize,
        edit_mode: bool,
        player: &Player,
        volume: f32,
    ) -> pane_grid::Content<'_, Message> {
        if edit_mode {
            let title = row![text(self.content.title()).size(14)]
                .spacing(5)
                .align_y(Vertical::Center);

            let close_button: Element<'_, Message> = button(text("X").size(14))
                .style(button::danger)
                .padding(3)
                .on_press_maybe(if total_panes > 1 {
                    Some(Message::Close(pane))
                } else {
                    None
                })
                .into();

            let title_bar = pane_grid::TitleBar::new(title)
                .controls(close_button)
                .padding(10)
                .style(|theme: &Theme| {
                    let palette = theme.extended_palette();
                    container::Style {
                        text_color: Some(palette.background.strong.text),
                        background: Some(palette.background.strong.color.into()),
                        ..Default::default()
                    }
                });

            let button_style = |label, message| {
                button(text(label).align_x(Horizontal::Center).size(16))
                    .width(Length::Fill)
                    .padding(8)
                    .on_press(message)
            };

            let mut controls = column![
                button_style(
                    "Split horizontally",
                    Message::Split(pane_grid::Axis::Horizontal, pane),
                ),
                button_style(
                    "Split vertically",
                    Message::Split(pane_grid::Axis::Vertical, pane),
                ),
            ]
            .spacing(5)
            .max_width(160)
            .align_x(Horizontal::Center);

            if total_panes > 1 {
                controls = controls.push(
                    button(text("Close").align_x(Horizontal::Center).size(16))
                        .width(Length::Fill)
                        .padding(8)
                        .style(button::danger)
                        .on_press(Message::Close(pane)),
                );
            }

            let edit_content = container(column![controls].spacing(10).align_x(Horizontal::Center))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .padding(5)
                .style(|theme: &Theme| {
                    let palette = theme.extended_palette();
                    container::Style {
                        background: Some(palette.background.weak.color.into()),
                        border: Border {
                            width: 2.0,
                            color: palette.background.strong.color,
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                });

            pane_grid::Content::new(edit_content).title_bar(title_bar)
        } else {
            pane_grid::Content::new(self.render_content(player, volume))
        }
    }

    fn render_content(&self, player: &Player, volume: f32) -> Element<'_, Message> {
        let content = match self.content {
            PaneContent::PlayerControls => {
                elements::player_controls::view(player, volume).map(Message::PlayerControls)
            }
            PaneContent::Queue => elements::queue::view(),
            PaneContent::Library => elements::library::view(),
            PaneContent::Artwork => elements::artwork::view(player),
            PaneContent::Timeline => elements::timeline::view(),
            PaneContent::Empty => elements::empty::view(),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
