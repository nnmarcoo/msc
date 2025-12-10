use iced::widget::{button, column, container, text};
use iced::{Element, Length, Theme};

/// Content types for different panes
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

/// Individual pane with content
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
        pane: iced::widget::pane_grid::Pane,
        total_panes: usize,
        edit_mode: bool,
    ) -> iced::widget::pane_grid::Content<'_, crate::layout::Message> {
        // In edit mode, show pane with title bar and simplified content
        if edit_mode {
            let close_button: Element<'_, crate::layout::Message> = if total_panes > 1 {
                button("✕")
                    .on_press(crate::layout::Message::Close(pane))
                    .padding([2, 6])
                    .style(|_theme: &Theme, status| {
                        match status {
                            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                                background: Some(iced::Color::from_rgb(0.8, 0.2, 0.2).into()),
                                text_color: iced::Color::WHITE,
                                ..Default::default()
                            },
                            _ => iced::widget::button::Style {
                                background: Some(iced::Color::from_rgb(0.3, 0.3, 0.3).into()),
                                text_color: iced::Color::from_rgb(0.8, 0.8, 0.8),
                                ..Default::default()
                            }
                        }
                    })
                    .into()
            } else {
                text("").into()
            };

            let title_bar = {
                use iced::widget::row;
                iced::widget::pane_grid::TitleBar::new(
                    row![
                        text(self.content.title()).size(14),
                    ]
                    .spacing(10)
                    .align_y(iced::alignment::Vertical::Center)
                )
                .controls(close_button)
                .padding(8)
            };

            // Create simplified edit mode content
            let edit_content = self.render_edit_content(pane, total_panes);

            iced::widget::pane_grid::Content::new(edit_content).title_bar(title_bar)
        } else {
            // Normal mode: show actual content without title bar
            let content = self.render_content();
            iced::widget::pane_grid::Content::new(content)
        }
    }

    fn render_edit_content(
        &self,
        pane: iced::widget::pane_grid::Pane,
        total_panes: usize,
    ) -> Element<'_, crate::layout::Message> {
        let delete_button: Element<'_, crate::layout::Message> = if total_panes > 1 {
            button("Delete")
                .on_press(crate::layout::Message::Close(pane))
                .padding([8, 16])
                .style(|_theme: &Theme, status| {
                    match status {
                        iced::widget::button::Status::Hovered => iced::widget::button::Style {
                            background: Some(iced::Color::from_rgb(0.8, 0.2, 0.2).into()),
                            text_color: iced::Color::WHITE,
                            ..Default::default()
                        },
                        _ => iced::widget::button::Style {
                            background: Some(iced::Color::from_rgb(0.4, 0.4, 0.4).into()),
                            text_color: iced::Color::from_rgb(0.9, 0.9, 0.9),
                            ..Default::default()
                        }
                    }
                })
                .into()
        } else {
            text("(Last pane)").size(12).color(iced::Color::from_rgb(0.5, 0.5, 0.5)).into()
        };

        container(
            column![
                text(self.content.title()).size(24),
                delete_button
            ]
            .spacing(20)
            .align_x(iced::alignment::Horizontal::Center)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Color::from_rgb(0.12, 0.12, 0.12).into()),
            ..Default::default()
        })
        .into()
    }

    fn render_content(&self) -> Element<'_, crate::layout::Message> {
        let content_text = match self.content {
            PaneContent::PlayerControls => {
                // Placeholder for player controls
                column![
                    text("▶ Play / ⏸ Pause").size(20),
                    text("⏮ Previous / ⏭ Next").size(20),
                    text("Volume Control").size(16),
                ]
                .spacing(10)
                .padding(20)
            }
            PaneContent::Queue => {
                // Placeholder for queue
                column![
                    text("Track 1 - Artist Name").size(14),
                    text("Track 2 - Artist Name").size(14),
                    text("Track 3 - Artist Name").size(14),
                ]
                .spacing(5)
                .padding(20)
            }
            PaneContent::Library => {
                // Placeholder for library
                column![
                    text("Library Browser").size(18),
                    text("Albums / Artists / Tracks").size(14),
                ]
                .spacing(10)
                .padding(20)
            }
            PaneContent::Artwork => {
                // Placeholder for artwork
                column![
                    container(text("🎵 Album Art").size(32))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x(Length::Fill)
                        .center_y(Length::Fill)
                ]
            }
            PaneContent::Timeline => {
                // Placeholder for timeline
                column![
                    text("Timeline / Seek Bar").size(16),
                    text("0:00 ━━━━━━━━━━ 3:45").size(14),
                ]
                .spacing(10)
                .padding(20)
            }
            PaneContent::Empty => column![text("Empty Pane").size(14)].spacing(5).padding(20),
        };

        container(content_text)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
