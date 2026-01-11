use iced::alignment::Vertical;
use iced::widget::svg::Handle;
use iced::widget::{button, container, pane_grid, pick_list, responsive, row, svg, text};
use iced::{Border, Element, Length, Theme};
use msc_core::{Player, Track};
use std::cell::RefCell;
use std::fmt::{self, Display};

use crate::app::Message;
use crate::pane_view::PaneView;
use crate::panes::*;
use crate::widgets::square_button::square_button;

// in edit mode the panes should be changed with a right click menu instead of the drop down!!!

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneType {
    Controls,
    Queue,
    Library,
    Collections,
    Artwork,
    Timeline,
    Spectrum,
    VUMeters,
    TrackInfo,
    Empty,
    Settings,
}

impl PaneType {
    pub const ALL: [PaneType; 11] = [
        PaneType::Controls,
        PaneType::Queue,
        PaneType::Library,
        PaneType::Collections,
        PaneType::Artwork,
        PaneType::Timeline,
        PaneType::Spectrum,
        PaneType::VUMeters,
        PaneType::TrackInfo,
        PaneType::Empty,
        PaneType::Settings,
    ];

    pub fn title(&self) -> &str {
        match self {
            PaneType::Controls => "Controls",
            PaneType::Queue => "Queue",
            PaneType::Library => "Library",
            PaneType::Collections => "Collections",
            PaneType::Artwork => "Artwork",
            PaneType::Timeline => "Timeline",
            PaneType::Spectrum => "Spectrum",
            PaneType::VUMeters => "VU Meters",
            PaneType::TrackInfo => "Track Info",
            PaneType::Empty => "Empty",
            PaneType::Settings => "Settings",
        }
    }

    pub fn create(&self) -> Box<dyn PaneView> {
        match self {
            PaneType::Controls => Box::new(ControlsPane::new()),
            PaneType::Queue => Box::new(QueuePane::new()),
            PaneType::Library => Box::new(LibraryPane::new()),
            PaneType::Collections => Box::new(CollectionsPane::new()),
            PaneType::Artwork => Box::new(ArtworkPane::new()),
            PaneType::Timeline => Box::new(TimelinePane::new()),
            PaneType::Spectrum => Box::new(SpectrumPane::new()),
            PaneType::VUMeters => Box::new(VUMetersPane::new()),
            PaneType::TrackInfo => Box::new(TrackInfoPane::new()),
            PaneType::Empty => Box::new(EmptyPane::new()),
            PaneType::Settings => Box::new(SettingsPane::new()),
        }
    }
}

impl Display for PaneType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.title())
    }
}

#[derive(Debug, Clone)]
pub struct Pane {
    pub content: Box<dyn PaneView>,
}

impl Pane {
    pub fn new(pane_type: PaneType) -> Self {
        Self {
            content: pane_type.create(),
        }
    }

    pub fn set_content(&mut self, pane_type: PaneType) {
        self.content = pane_type.create();
    }

    pub fn update(&mut self, player: &Player) {
        self.content.update(player);
    }

    pub fn get_type(&self) -> PaneType {
        let title = self.content.title();
        match title {
            "Controls" => PaneType::Controls,
            "Queue" => PaneType::Queue,
            "Library" => PaneType::Library,
            "Collections" => PaneType::Collections,
            "Artwork" => PaneType::Artwork,
            "Timeline" => PaneType::Timeline,
            "Spectrum" => PaneType::Spectrum,
            "VU Meters" => PaneType::VUMeters,
            "Track Info" => PaneType::TrackInfo,
            "Settings" => PaneType::Settings,
            _ => PaneType::Empty,
        }
    }
    pub fn view<'a>(
        &'a self,
        pane: pane_grid::Pane,
        total_panes: usize,
        edit_mode: bool,
        player: &'a Player,
        volume: f32,
        hovered_track: &Option<i64>,
        seeking_position: Option<f32>,
        cached_tracks: &'a RefCell<Option<Vec<Track>>>,
        cached_albums: &'a RefCell<
            Option<Vec<(i64, String, Option<String>, Option<u32>, Option<String>)>>,
        >,
    ) -> pane_grid::Content<'a, Message> {
        if edit_mode {
            let current_type = self.get_type();
            let title = row![
                pick_list(&PaneType::ALL[..], Some(current_type), move |pane_type| {
                    Message::PaneTypeChanged(pane, pane_type)
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
                            text_color: Some(palette.background.weak.text),
                            background: Some(palette.background.base.color.into()),
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
            let content = self.content.view(
                player,
                volume,
                hovered_track,
                seeking_position,
                cached_tracks,
                cached_albums,
            );

            pane_grid::Content::new(
                container(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
        }
    }
}
