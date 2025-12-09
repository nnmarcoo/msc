use iced::widget::pane_grid::{self, Highlight, PaneGrid};
use iced::{Background, Border, Color, Element, Length, Subscription, Task};
use std::time::Duration;

use crate::pane::{Pane, PaneContent};

pub struct Layout {
    panes: pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    maximized: Option<pane_grid::Pane>,
    edit_mode: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Split(pane_grid::Axis, pane_grid::Pane),
    Close(pane_grid::Pane),
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    ToggleMaximize(pane_grid::Pane),
    ToggleEditMode,
    Tick,
}

impl Default for Layout {
    fn default() -> Self {
        let pane_config = pane_grid::Configuration::Split {
            axis: pane_grid::Axis::Vertical,
            ratio: 0.6,
            a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: 0.7,
                a: Box::new(pane_grid::Configuration::Pane(Pane::new(
                    PaneContent::Artwork,
                ))),
                b: Box::new(pane_grid::Configuration::Pane(Pane::new(
                    PaneContent::PlayerControls,
                ))),
            }),
            b: Box::new(pane_grid::Configuration::Pane(Pane::new(
                PaneContent::Queue,
            ))),
        };

        let panes = pane_grid::State::with_configuration(pane_config);

        Self {
            panes,
            focus: None,
            maximized: None,
            edit_mode: false,
        }
    }
}

impl Layout {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Split(axis, pane) => {
                if let Some((new_pane, _)) = self.panes.split(axis, pane, Pane::new(PaneContent::Empty)) {
                    self.focus = Some(new_pane);
                }
            }
            Message::Close(pane) => {
                if let Some((_, sibling)) = self.panes.close(pane) {
                    self.focus = Some(sibling);
                }

                if self.maximized == Some(pane) {
                    self.maximized = None;
                }
            }
            Message::Clicked(pane) => {
                self.focus = Some(pane);
            }
            Message::Dragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                self.panes.drop(pane, target);
            }
            Message::Dragged(_) => {}
            Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);
            }
            Message::ToggleMaximize(pane) => {
                if self.maximized == Some(pane) {
                    self.maximized = None;
                } else {
                    self.maximized = Some(pane);
                    self.focus = Some(pane);
                }
            }
            Message::ToggleEditMode => {
                self.edit_mode = !self.edit_mode;
            }
            Message::Tick => {
                // player state updates, etc.
            }
        }

        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        use iced::widget::{button, column, container};

        let total_panes = self.panes.len();
        let focus = self.focus;
        let maximized = self.maximized;
        let edit_mode = self.edit_mode;

        let top_bar = if edit_mode {
            use iced::widget::{row, text};
            container(
                row![
                    text("Edit Mode: Drag panes from ANYWHERE • Drag borders to resize • Click Delete to remove")
                        .size(14),
                    button("✓ Done")
                        .on_press(Message::ToggleEditMode)
                        .padding(10)
                ]
                .spacing(20)
                .align_y(iced::alignment::Vertical::Center)
            )
            .width(Length::Fill)
            .padding(10)
            .align_x(iced::alignment::Horizontal::Right)
        } else {
            container(
                button("⚙ Edit Layout")
                    .on_press(Message::ToggleEditMode)
                    .padding(10)
            )
            .width(Length::Fill)
            .padding(10)
            .align_x(iced::alignment::Horizontal::Right)
        };

        let mut pane_grid = PaneGrid::new(&self.panes, move |id, pane, _maximized_in_grid| {
            let _is_focused = focus == Some(id);
            let is_maximized = maximized == Some(id);

            pane.view(id, total_panes, is_maximized, edit_mode)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(if edit_mode { 4 } else { 0 })
;

        // Only enable interactions in edit mode
        if edit_mode {
            pane_grid = pane_grid
                .on_click(Message::Clicked)
                .on_drag(Message::Dragged)
                .on_resize(10, Message::Resized);
        }

        column![
            top_bar,
            pane_grid,
        ]
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick)
    }
}
