use blake3::Hash;
use iced::keyboard::{self, Key};
use iced::time::every;
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{column, container};
use iced::{Element, Event, Length, Subscription, Task, Theme};
use msc_core::Player;
use std::path::PathBuf;
use std::time::Duration;

use crate::components::{bottom_bar, controls};
use crate::pane::{Pane, PaneContent};

pub struct App {
    panes: pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    edit_mode: bool,
    player: Player,
    volume: f32,
    previous_volume: f32,
    seeking_position: Option<f32>,
    layout_presets: Vec<pane_grid::Configuration<Pane>>,
    current_preset: usize,
    hovered_track: Option<Hash>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Split(pane_grid::Axis, pane_grid::Pane),
    Close(pane_grid::Pane),
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    Controls(controls::Message),
    LibraryPathSelected(Option<PathBuf>),
    PaneContentChanged(pane_grid::Pane, PaneContent),
    BottomBar(bottom_bar::Message),
    PlayTrack(Hash),
    QueueBack(Hash),
    QueueFront(Hash),
    TrackHovered(Hash),
    TrackUnhovered,
    Event(Event),
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
                    PaneContent::Library,
                ))),
                b: Box::new(pane_grid::Configuration::Split {
                    axis: pane_grid::Axis::Horizontal,
                    ratio: 0.5,
                    a: Box::new(pane_grid::Configuration::Pane(Pane::new(
                        PaneContent::Artwork,
                    ))),
                    b: Box::new(pane_grid::Configuration::Pane(Pane::new(
                        PaneContent::Queue,
                    ))),
                }),
            }),
            b: Box::new(pane_grid::Configuration::Pane(Pane::new(
                PaneContent::Controls,
            ))),
        };

        let panes = pane_grid::State::with_configuration(pane_config.clone());
        let player = Player::new().expect("Failed to initialize player");

        Self {
            panes,
            focus: None,
            edit_mode: false,
            player,
            volume: 0.5,
            previous_volume: 0.5,
            seeking_position: None,
            layout_presets: vec![pane_config],
            current_preset: 0,
            hovered_track: None,
        }
    }
}

impl App {
    fn save_current_layout(&mut self) {
        let config = self.panes.layout().clone();
        self.layout_presets[self.current_preset] =
            Self::layout_to_configuration(&self.panes, config);
    }

    fn layout_to_configuration(
        panes: &pane_grid::State<Pane>,
        layout: pane_grid::Node,
    ) -> pane_grid::Configuration<Pane> {
        match layout {
            pane_grid::Node::Split {
                axis, ratio, a, b, ..
            } => pane_grid::Configuration::Split {
                axis,
                ratio,
                a: Box::new(Self::layout_to_configuration(panes, *a)),
                b: Box::new(Self::layout_to_configuration(panes, *b)),
            },
            pane_grid::Node::Pane(pane_id) => {
                let pane = panes.get(pane_id).unwrap();
                pane_grid::Configuration::Pane(pane.clone())
            }
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
            Message::Tick => {
                let _ = self.player.update();
            }
            Message::LibraryPathSelected(path) => {
                if let Some(path) = path {
                    let _ = self.player.populate_library(&path);
                }
            }
            Message::Controls(msg) => {
                use controls::Message as ControlsMessage;
                match msg {
                    ControlsMessage::PlayPause => {
                        if self.player.is_playing() {
                            self.player.pause();
                        } else {
                            let _ = self.player.play();
                        }
                    }
                    ControlsMessage::Previous => {
                        let _ = self.player.start_previous();
                    }
                    ControlsMessage::Next => {
                        let _ = self.player.start_next();
                    }
                    ControlsMessage::VolumeChanged(vol) => {
                        self.volume = vol;
                        self.player.set_volume(vol);
                    }
                    ControlsMessage::ToggleMute => {
                        if self.volume > 0.0 {
                            self.previous_volume = self.volume;
                            self.volume = 0.0;
                        } else {
                            self.volume = self.previous_volume;
                        }
                        self.player.set_volume(self.volume);
                    }
                    ControlsMessage::SeekChanged(pos) => {
                        self.seeking_position = Some(pos);
                    }
                    ControlsMessage::SeekReleased => {
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
            Message::BottomBar(msg) => {
                use bottom_bar::Message as BottomBarMessage;
                match msg {
                    BottomBarMessage::LoadLibrary => {
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
                    BottomBarMessage::QueueLibrary => {
                        self.player.queue_library();
                        let _ = self.player.play();
                    }
                    BottomBarMessage::ClearQueue => {
                        self.player.clear_queue();
                    }
                    BottomBarMessage::ShuffleQueue => {
                        self.player.shuffle_queue();
                    }
                    BottomBarMessage::ToggleEditMode => {
                        if self.edit_mode {
                            self.save_current_layout();
                        }
                        self.edit_mode = !self.edit_mode;
                    }
                    BottomBarMessage::SwitchPreset(index) => {
                        if index < self.layout_presets.len() {
                            if self.edit_mode {
                                self.save_current_layout();
                            }

                            self.current_preset = index;
                            self.panes = pane_grid::State::with_configuration(
                                self.layout_presets[index].clone(),
                            );
                            self.focus = None;
                        }
                    }
                    BottomBarMessage::AddPreset => {
                        let new_preset =
                            pane_grid::Configuration::Pane(Pane::new(PaneContent::Empty));
                        self.layout_presets.push(new_preset.clone());
                        self.current_preset = self.layout_presets.len() - 1;
                        self.panes = pane_grid::State::with_configuration(new_preset);
                        self.focus = None;
                    }
                }
            }
            Message::PlayTrack(track_id) => {
                if let Some(_track) = self.player.library().track_from_id(track_id) {
                    self.player.queue_front(track_id);
                    let _ = self.player.start_next();
                }
            }
            Message::QueueBack(track_id) => {
                if let Some(_track) = self.player.library().track_from_id(track_id) {
                    self.player.queue_back(track_id);
                }
            }
            Message::QueueFront(track_id) => {
                if let Some(_track) = self.player.library().track_from_id(track_id) {
                    self.player.queue_front(track_id);
                }
            }
            Message::TrackHovered(track_id) => {
                self.hovered_track = Some(track_id);
            }
            Message::TrackUnhovered => {
                self.hovered_track = None;
            }
            Message::Event(event) => {
                if let Event::Keyboard(keyboard::Event::KeyPressed {
                    key, modifiers: _, ..
                }) = event
                {
                    match key {
                        Key::Character(c) => {
                            if let Ok(num) = c.parse::<usize>() {
                                if num >= 1 && num <= self.layout_presets.len() {
                                    let index = num - 1;
                                    if self.edit_mode {
                                        self.save_current_layout();
                                    }
                                    self.current_preset = index;
                                    self.panes = pane_grid::State::with_configuration(
                                        self.layout_presets[index].clone(),
                                    );
                                    self.focus = None;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        let total_panes = self.panes.len();
        let edit_mode = self.edit_mode;

        let player = &self.player;
        let volume = self.volume;
        let hovered_track = &self.hovered_track;
        let mut pane_grid = PaneGrid::new(&self.panes, move |id, pane, _is_maximized| {
            pane.view(id, total_panes, edit_mode, player, volume, hovered_track)
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

        column![
            pane_grid_container,
            bottom_bar::view(
                self.layout_presets.len(),
                self.current_preset,
                self.edit_mode,
            )
            .map(Message::BottomBar)
        ]
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            every(Duration::from_millis(30)).map(|_| Message::Tick),
            iced::event::listen().map(Message::Event),
        ])
    }
}
