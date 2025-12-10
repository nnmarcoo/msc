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
        if edit_mode {
            use iced::widget::row;

            // Title bar with pane name
            let title = row![text(self.content.title()).size(14)]
                .spacing(5)
                .align_y(iced::alignment::Vertical::Center);

            // Close button (X) in title bar
            let close_button: Element<'_, crate::layout::Message> =
                button(text("X").size(14))
                    .style(button::danger)
                    .padding(3)
                    .on_press_maybe(if total_panes > 1 {
                        Some(crate::layout::Message::Close(pane))
                    } else {
                        None
                    })
                    .into();

            let title_bar = iced::widget::pane_grid::TitleBar::new(title)
                .controls(close_button)
                .padding(10)
                .style(|theme: &Theme| {
                    let palette = theme.extended_palette();
                    iced::widget::container::Style {
                        text_color: Some(palette.background.strong.text),
                        background: Some(palette.background.strong.color.into()),
                        ..Default::default()
                    }
                });

            // Content area with split and close buttons
            let button_style = |label, message| {
                button(text(label).align_x(iced::alignment::Horizontal::Center).size(16))
                    .width(Length::Fill)
                    .padding(8)
                    .on_press(message)
            };

            let mut controls_col = column![
                button_style(
                    "Split horizontally",
                    crate::layout::Message::Split(iced::widget::pane_grid::Axis::Horizontal, pane),
                ),
                button_style(
                    "Split vertically",
                    crate::layout::Message::Split(iced::widget::pane_grid::Axis::Vertical, pane),
                ),
            ]
            .spacing(5)
            .max_width(160)
            .align_x(iced::alignment::Horizontal::Center);

            if total_panes > 1 {
                controls_col = controls_col.push(
                    button(text("Close").align_x(iced::alignment::Horizontal::Center).size(16))
                        .width(Length::Fill)
                        .padding(8)
                        .style(button::danger)
                        .on_press(crate::layout::Message::Close(pane))
                );
            }

            let controls = controls_col;

            let edit_content = container(
                column![controls].spacing(10).align_x(iced::alignment::Horizontal::Center)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(5)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(palette.background.weak.color.into()),
                    border: iced::Border {
                        width: 2.0,
                        color: palette.background.strong.color,
                        ..Default::default()
                    },
                    ..Default::default()
                }
            });

            iced::widget::pane_grid::Content::new(edit_content).title_bar(title_bar)
        } else {
            // Normal mode: show actual content without title bar
            let content = self.render_content();
            iced::widget::pane_grid::Content::new(content)
        }
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
