use iced::widget::{Space, container, pane_grid, text};
use iced::{Length, Theme};
use msc_core::{Album, Player, Playlist, Track};
use std::cell::RefCell;
use std::fmt::{self, Display};

use crate::app::Message;
use crate::art_cache::ArtCache;
use crate::components::context_menu::{MenuElement, context_menu};
use crate::pane_view::{PaneView, ViewContext};
use crate::panes::*;

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
}

impl PaneType {
    pub const ALL: [PaneType; 10] = [
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
    ];

    pub fn title(&self) -> &'static str {
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
        }
    }

    pub fn from_title(s: &str) -> Self {
        Self::ALL
            .iter()
            .find(|t| t.title() == s)
            .copied()
            .unwrap_or(PaneType::Empty)
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
    pub pane_type: PaneType,
    pub content: Box<dyn PaneView>,
}

impl Pane {
    pub fn new(pane_type: PaneType) -> Self {
        Self {
            content: pane_type.create(),
            pane_type,
        }
    }

    pub fn set_content(&mut self, pane_type: PaneType) {
        self.pane_type = pane_type;
        self.content = pane_type.create();
    }

    pub fn update(&mut self, player: &Player, art: &mut ArtCache) {
        self.content.update(player, art);
    }

    pub fn invalidate_cache(&mut self) {
        self.content.invalidate_cache();
    }

    pub fn get_type(&self) -> PaneType {
        self.pane_type
    }

    pub fn view<'a>(
        &'a self,
        pane: pane_grid::Pane,
        total_panes: usize,
        edit_mode: bool,
        player: &'a Player,
        volume: f32,
        hovered_track: &'a Option<i64>,
        seeking_position: Option<f32>,
        cached_tracks: &'a RefCell<Option<Vec<Track>>>,
        cached_albums: &'a RefCell<Option<Vec<Album>>>,
        cached_playlists: &'a RefCell<Option<Vec<Playlist>>>,
        art: &'a ArtCache,
    ) -> pane_grid::Content<'a, Message> {
        if edit_mode {
            let current_type = self.get_type();

            let mut items: Vec<MenuElement<Message>> = PaneType::ALL
                .iter()
                .filter(|&&t| t != current_type)
                .map(|t| MenuElement::button(t.title(), Message::PaneTypeChanged(pane, *t)))
                .collect();

            items.push(MenuElement::Separator);
            items.push(MenuElement::button(
                "Split Horizontally",
                Message::Split(pane_grid::Axis::Horizontal, pane),
            ));
            items.push(MenuElement::button(
                "Split Vertically",
                Message::Split(pane_grid::Axis::Vertical, pane),
            ));

            if total_panes > 1 {
                items.push(MenuElement::Separator);
                items.push(MenuElement::button("Close", Message::Close(pane)));
            }

            let body =
                context_menu(
                    container(text(current_type.title()).size(16).style(|theme: &Theme| {
                        text::Style {
                            color: Some(theme.extended_palette().background.base.text),
                        }
                    }))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .style(|theme: &Theme| container::Style {
                        background: Some(theme.extended_palette().background.base.color.into()),
                        ..Default::default()
                    }),
                    items,
                );

            let title_bar = pane_grid::TitleBar::new(Space::new().height(0))
                .padding([9, 0])
                .style(|theme: &Theme| {
                    let palette = theme.extended_palette();
                    container::Style {
                        background: Some(palette.primary.base.color.into()),
                        ..Default::default()
                    }
                });

            pane_grid::Content::new(body).title_bar(title_bar)
        } else {
            let ctx = ViewContext {
                player,
                volume,
                hovered_track,
                seeking_position,
                cached_tracks,
                cached_albums,
                cached_playlists,
                art,
            };

            let content = self.content.view(ctx);

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
