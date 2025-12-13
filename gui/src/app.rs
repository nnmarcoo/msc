use iced::alignment::{Horizontal, Vertical};
use iced::time::every;
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::svg::Handle;
use iced::widget::{column, container, row, svg, text};
use iced::{Background, Element, Length, Subscription, Task, Theme};
use msc_core::Player;
use std::path::PathBuf;
use std::time::Duration;

use crate::components::controls;
use crate::pane::{Pane, PaneContent};
use crate::widgets::sharp_button::sharp_button;
use crate::widgets::square_button::square_button;

pub struct App {
    panes: pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    edit_mode: bool,
    player: Player,
    volume: f32,
    previous_volume: f32,
    seeking_position: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Split(pane_grid::Axis, pane_grid::Pane),
    Close(pane_grid::Pane),
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    ToggleEditMode,
    Tick,
    PlayerControls(controls::Message),
    LoadLibrary,
    LibraryPathSelected(Option<PathBuf>),
    QueueLibrary,
    PaneContentChanged(pane_grid::Pane, PaneContent),
}

impl Default for App {
    fn default() -> Self {
        let pane_config = pane_grid::Configuration::Split {
            axis: pane_grid::Axis::Horizontal,
            ratio: 0.9,
            a: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: 0.7,
                a: Box::new(pane_grid::Configuration::Pane(Pane::new(
                    PaneContent::Artwork,
                ))),
                b: Box::new(pane_grid::Configuration::Pane(Pane::new(
                    PaneContent::Queue,
                ))),
            }),
            b: Box::new(pane_grid::Configuration::Pane(Pane::new(
                PaneContent::Controls,
            ))),
        };

        let panes = pane_grid::State::with_configuration(pane_config);
        let player = Player::new().expect("Failed to initialize player");

        Self {
            panes,
            focus: None,
            edit_mode: false,
            player,
            volume: 0.5,
            previous_volume: 0.5,
            seeking_position: None,
        }
    }
}

impl App {
    fn bar_style(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();
        let mut color = palette.background.base.color;
        color.r = (color.r + 0.02).min(1.0);
        color.g = (color.g + 0.02).min(1.0);
        color.b = (color.b + 0.02).min(1.0);
        container::Style {
            text_color: Some(palette.background.base.text),
            background: Some(Background::Color(color)),
            ..Default::default()
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Split(axis, pane) => {
                if let Some((new_pane, _)) =
                    self.panes.split(axis, pane, Pane::new(PaneContent::Empty))
                {
                    self.focus = Some(new_pane);
                }
            }
            Message::Close(pane) => {
                if let Some((_, sibling)) = self.panes.close(pane) {
                    self.focus = Some(sibling);
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
            Message::ToggleEditMode => {
                self.edit_mode = !self.edit_mode;
            }
            Message::Tick => {
                let _ = self.player.update();
            }
            Message::LoadLibrary => {
                if self.player.reload_library().is_err() {
                    return Task::perform(
                        async {
                            rfd::AsyncFileDialog::new()
                                .set_title("Select Music Library Folder")
                                .pick_folder()
                                .await
                                .map(|handle| handle.path().to_path_buf())
                        },
                        Message::LibraryPathSelected,
                    );
                }
            }
            Message::LibraryPathSelected(path) => {
                if let Some(path) = path {
                    self.player.populate_library(&path);
                }
            }
            Message::QueueLibrary => {
                self.player.queue_library();
                let _ = self.player.play();
            }
            Message::PlayerControls(msg) => {
                use controls::Message as PCMsg;
                match msg {
                    PCMsg::PlayPause => {
                        if self.player.is_playing() {
                            self.player.pause();
                        } else {
                            let _ = self.player.play();
                        }
                    }
                    PCMsg::Previous => {
                        let _ = self.player.start_previous();
                    }
                    PCMsg::Next => {
                        let _ = self.player.start_next();
                    }
                    PCMsg::VolumeChanged(vol) => {
                        self.volume = vol;
                        self.player.set_volume(vol);
                    }
                    PCMsg::ToggleMute => {
                        if self.volume > 0.0 {
                            self.previous_volume = self.volume;
                            self.volume = 0.0;
                        } else {
                            self.volume = self.previous_volume;
                        }
                        self.player.set_volume(self.volume);
                    }
                    PCMsg::SeekChanged(pos) => {
                        self.seeking_position = Some(pos);
                    }
                    PCMsg::SeekReleased => {
                        if let Some(pos) = self.seeking_position {
                            self.player.seek(pos as f64);
                            self.seeking_position = None;
                        }
                    }
                }
            }
            Message::PaneContentChanged(pane_id, new_content) => {
                if let Some(pane) = self.panes.get_mut(pane_id) {
                    pane.set_content(new_content);
                }
            }
        }

        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        let total_panes = self.panes.len();
        let edit_mode = self.edit_mode;
        let bottom_bar = if edit_mode {
            container(
                row![
                    square_button(
                        svg(Handle::from_memory(include_bytes!(
                            "../../assets/icons/checkmark.svg"
                        ))),
                        20
                    )
                    .on_press(Message::ToggleEditMode)
                ]
                .spacing(5)
                .align_y(Vertical::Center),
            )
            .width(Length::Fill)
            .align_x(Horizontal::Right)
            .style(Self::bar_style)
        } else {
            container(
                row![
                    sharp_button("loadlib")
                        .height(20)
                        .on_press(Message::LoadLibrary),
                    sharp_button("quelib")
                        .height(20)
                        .on_press(Message::QueueLibrary),
                    square_button(
                        svg(Handle::from_memory(include_bytes!(
                            "../../assets/icons/settings.svg"
                        ))),
                        20
                    )
                    .on_press(Message::ToggleEditMode),
                ]
                .spacing(5),
            )
            .width(Length::Fill)
            .align_x(Horizontal::Right)
            .style(Self::bar_style)
        };

        let player = &self.player;
        let volume = self.volume;
        let mut pane_grid = PaneGrid::new(&self.panes, move |id, pane, _is_maximized| {
            pane.view(id, total_panes, edit_mode, player, volume)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(if edit_mode { 4 } else { 0 });

        if edit_mode {
            pane_grid = pane_grid
                .on_click(Message::Clicked)
                .on_drag(Message::Dragged)
                .on_resize(10, Message::Resized);
        }

        let pane_grid_container = if edit_mode {
            container(pane_grid)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|theme: &Theme| {
                    let palette = theme.extended_palette();
                    container::Style {
                        background: Some(palette.background.weak.color.into()),
                        ..Default::default()
                    }
                })
        } else {
            container(pane_grid)
                .width(Length::Fill)
                .height(Length::Fill)
        };

        column![pane_grid_container, bottom_bar].into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        every(Duration::from_millis(250)).map(|_| Message::Tick)
    }
}
