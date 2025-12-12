use iced::alignment::{Horizontal, Vertical};
use iced::widget::svg::Handle;
use iced::widget::{button, container, pane_grid, pick_list, responsive, row, svg, text};
use iced::{Border, Element, Length, Theme};
use msc_core::Player;
use std::fmt::{self, Display};

use crate::app::Message;
use crate::components;
use crate::widgets::square_button::square_button;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneContent {
    Controls,
    Queue,
    Library,
    Artwork,
    Timeline,
    Empty,
}

impl PaneContent {
    pub const ALL: [PaneContent; 6] = [
        PaneContent::Controls,
        PaneContent::Queue,
        PaneContent::Library,
        PaneContent::Artwork,
        PaneContent::Timeline,
        PaneContent::Empty,
    ];

    pub fn title(&self) -> &str {
        match self {
            PaneContent::Controls => "Controls",
            PaneContent::Queue => "Queue",
            PaneContent::Library => "Library",
            PaneContent::Artwork => "Artwork",
            PaneContent::Timeline => "Timeline",
            PaneContent::Empty => "Empty",
        }
    }
}

impl Display for PaneContent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.title())
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

    pub fn set_content(&mut self, content: PaneContent) {
        self.content = content;
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
            let title = row![
                pick_list(&PaneContent::ALL[..], Some(self.content), move |content| {
                    Message::PaneContentChanged(pane, content)
                })
                .text_size(14)
            ]
            .spacing(5)
            .align_y(Vertical::Center);

            let horizontal_split: Element<'_, Message> = square_button(
                svg(Handle::from_memory(include_bytes!(
                    "../../assets/icons/horizontal.svg"
                ))),
                20,
            )
            .style(button::primary)
            .on_press(Message::Split(pane_grid::Axis::Horizontal, pane))
            .into();

            let vertical_split: Element<'_, Message> = square_button(
                svg(Handle::from_memory(include_bytes!(
                    "../../assets/icons/vertical.svg"
                ))),
                20,
            )
            .style(button::primary)
            .on_press(Message::Split(pane_grid::Axis::Vertical, pane))
            .into();

            let mut controls = row![horizontal_split, vertical_split]
                .spacing(5)
                .align_y(Vertical::Center);

            if total_panes > 1 {
                let close_btn: Element<'_, Message> = button(svg(Handle::from_memory(
                    include_bytes!("../../assets/icons/x.svg"),
                )))
                .width(Length::Fixed(20.0))
                .height(Length::Fixed(20.0))
                .padding(0)
                .style(button::danger)
                .on_press(Message::Close(pane))
                .into();

                controls = controls.push(close_btn);
            }

            let controls_element: Element<'_, Message> = container(controls).into();

            let title_bar = pane_grid::TitleBar::new(title)
                .controls(controls_element)
                .style(|theme: &Theme| {
                    let palette = theme.extended_palette();
                    container::Style {
                        text_color: Some(palette.background.strong.text),
                        background: Some(palette.background.strong.color.into()),
                        ..Default::default()
                    }
                });

            let edit_content = responsive(move |size| {
                let size_text = format!("{}x{}", size.width as u32, size.height as u32);
                container(text(size_text).size(16))
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
                    })
                    .into()
            });

            pane_grid::Content::new(edit_content).title_bar(title_bar)
        } else {
            pane_grid::Content::new(self.render_content(player, volume))
        }
    }

    fn render_content(&self, player: &Player, volume: f32) -> Element<'_, Message> {
        let content = match self.content {
            PaneContent::Controls => {
                components::player_controls::view(player, volume).map(Message::PlayerControls)
            }
            PaneContent::Queue => components::queue::view(player),
            PaneContent::Library => components::library::view(player),
            PaneContent::Artwork => components::artwork::view(player),
            PaneContent::Timeline => components::timeline::view(),
            PaneContent::Empty => components::empty::view(),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
