//! What a pane can contain.
//!
//! [`PaneKind`] is an enum rather than a trait object, and per-pane state lives
//! in [`PaneState`] keyed by [`PaneId`]. Reaching one pane's state is a match,
//! not a downcast, so adding a kind fails to compile everywhere that must
//! handle it instead of silently doing nothing at runtime.
//!
//! The per-kind state, messages, and dispatch are in place but not yet driven
//! for most kinds: panes render as labels while the layout mechanics are the
//! focus. The scaffolding stays so that wiring real content later is a
//! self-contained change rather than a rework.
//!
//! [`PaneState`] holds only what two panes of the same kind must be able to
//! disagree about. Anything keyed on a track id, such as the search query, the
//! selection or the hovered row, is shared across panes and lives in
//! [`crate::tracks`] instead, which is why the library pane has no state here.
//! The queue's history toggle and the timeline's remaining-time toggle both
//! qualify: each is about how one pane draws itself and nothing else.
#![allow(dead_code)]

pub mod artwork;
pub mod controls;
pub mod library;
pub mod queue;
pub mod search;
pub mod timeline;
pub mod view;
pub mod volume;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::layout::PaneId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaneCategory {
    Browse,
    Playback,
    Detail,
    Tools,
}

impl PaneCategory {
    pub fn title(self) -> &'static str {
        match self {
            PaneCategory::Browse => "Browse",
            PaneCategory::Playback => "Playback",
            PaneCategory::Detail => "Detail",
            PaneCategory::Tools => "Tools",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneKind {
    Library,
    Search,
    Albums,
    Artists,
    Playlists,
    Folders,
    Queue,
    NowPlaying,
    Controls,
    Timeline,
    Volume,
    History,
    Lyrics,
    TrackInfo,
    Artwork,
    Visualiser,
    Equaliser,
    Settings,
    Empty,
}

impl PaneKind {
    pub const ALL: [PaneKind; 19] = [
        PaneKind::Library,
        PaneKind::Search,
        PaneKind::Albums,
        PaneKind::Artists,
        PaneKind::Playlists,
        PaneKind::Folders,
        PaneKind::Queue,
        PaneKind::NowPlaying,
        PaneKind::Controls,
        PaneKind::Timeline,
        PaneKind::Volume,
        PaneKind::History,
        PaneKind::Lyrics,
        PaneKind::TrackInfo,
        PaneKind::Artwork,
        PaneKind::Visualiser,
        PaneKind::Equaliser,
        PaneKind::Settings,
        PaneKind::Empty,
    ];

    pub fn title(self) -> &'static str {
        match self {
            PaneKind::Library => "Library",
            PaneKind::Search => "Search",
            PaneKind::Albums => "Albums",
            PaneKind::Artists => "Artists",
            PaneKind::Playlists => "Playlists",
            PaneKind::Folders => "Folders",
            PaneKind::Queue => "Queue",
            PaneKind::NowPlaying => "Now Playing",
            PaneKind::Controls => "Controls",
            PaneKind::Timeline => "Timeline",
            PaneKind::Volume => "Volume",
            PaneKind::History => "History",
            PaneKind::Lyrics => "Lyrics",
            PaneKind::TrackInfo => "Track Info",
            PaneKind::Artwork => "Artwork",
            PaneKind::Visualiser => "Visualiser",
            PaneKind::Equaliser => "Equaliser",
            PaneKind::Settings => "Settings",
            PaneKind::Empty => "Empty",
        }
    }

    pub fn category(self) -> PaneCategory {
        match self {
            PaneKind::Library
            | PaneKind::Search
            | PaneKind::Albums
            | PaneKind::Artists
            | PaneKind::Playlists
            | PaneKind::Folders => PaneCategory::Browse,
            PaneKind::Queue
            | PaneKind::NowPlaying
            | PaneKind::Controls
            | PaneKind::Timeline
            | PaneKind::Volume
            | PaneKind::History => PaneCategory::Playback,
            PaneKind::Lyrics | PaneKind::TrackInfo | PaneKind::Artwork | PaneKind::Visualiser => {
                PaneCategory::Detail
            }
            PaneKind::Equaliser | PaneKind::Settings | PaneKind::Empty => PaneCategory::Tools,
        }
    }

    pub fn keywords(self) -> &'static str {
        match self {
            PaneKind::Library => "songs tracks music collection",
            PaneKind::Search => "filter find query lookup",
            PaneKind::Albums => "records releases discography",
            PaneKind::Artists => "bands performers musicians",
            PaneKind::Playlists => "mixes sets collections",
            PaneKind::Folders => "files directories browse disk",
            PaneKind::Queue => "up next playlist upcoming",
            PaneKind::NowPlaying => "current track player",
            PaneKind::Controls => "transport play pause next previous skip",
            PaneKind::Timeline => "seek bar scrub position progress elapsed",
            PaneKind::Volume => "loudness level mute gain slider sound",
            PaneKind::History => "recent played log past",
            PaneKind::Lyrics => "words text karaoke",
            PaneKind::TrackInfo => "metadata tags details properties",
            PaneKind::Artwork => "cover art album picture image sleeve",
            PaneKind::Visualiser => "spectrum waveform graphics visualizer",
            PaneKind::Equaliser => "eq bands tone audio equalizer",
            PaneKind::Settings => "preferences options config",
            PaneKind::Empty => "blank none clear placeholder",
        }
    }

    fn matches(self, query_lower: &str) -> bool {
        self.title().to_lowercase().contains(query_lower)
            || self.keywords().contains(query_lower)
            || self.category().title().to_lowercase().contains(query_lower)
    }

    pub fn by_category() -> Vec<(PaneCategory, Vec<PaneKind>)> {
        let mut groups: Vec<(PaneCategory, Vec<PaneKind>)> = Vec::new();
        for kind in PaneKind::ALL {
            match groups.last_mut() {
                Some((category, kinds)) if *category == kind.category() => kinds.push(kind),
                _ => groups.push((kind.category(), vec![kind])),
            }
        }
        groups
    }

    pub fn search(query: &str) -> Vec<PaneKind> {
        let query_lower = query.to_lowercase();
        PaneKind::ALL
            .into_iter()
            .filter(|kind| kind.matches(&query_lower))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub enum PaneMessage {
    Queue(queue::Message),
    Timeline(timeline::Message),
}

#[derive(Debug)]
pub enum PaneState {
    Queue(queue::State),
    Timeline(timeline::State),
    Artwork(artwork::State),
    Stateless,
}

impl PaneState {
    fn for_kind(kind: PaneKind) -> Self {
        match kind {
            PaneKind::Queue => Self::Queue(queue::State::default()),
            PaneKind::Timeline => Self::Timeline(timeline::State::default()),
            PaneKind::Artwork => Self::Artwork(artwork::State::default()),
            PaneKind::Library
            | PaneKind::Search
            | PaneKind::Albums
            | PaneKind::Artists
            | PaneKind::Playlists
            | PaneKind::Folders
            | PaneKind::NowPlaying
            | PaneKind::Controls
            | PaneKind::Volume
            | PaneKind::History
            | PaneKind::Lyrics
            | PaneKind::TrackInfo
            | PaneKind::Visualiser
            | PaneKind::Equaliser
            | PaneKind::Settings
            | PaneKind::Empty => Self::Stateless,
        }
    }
}

#[derive(Debug, Default)]
pub struct PaneStates {
    states: HashMap<PaneId, PaneState>,
}

impl PaneStates {
    pub fn get_mut(&mut self, id: PaneId) -> Option<&mut PaneState> {
        self.states.get_mut(&id)
    }

    pub fn get(&self, id: PaneId) -> Option<&PaneState> {
        self.states.get(&id)
    }

    pub fn ensure(&mut self, id: PaneId, kind: PaneKind) {
        self.states
            .entry(id)
            .or_insert_with(|| PaneState::for_kind(kind));
    }

    pub fn reset(&mut self, id: PaneId, kind: PaneKind) {
        self.states.insert(id, PaneState::for_kind(kind));
    }

    pub fn remove(&mut self, id: PaneId) {
        self.states.remove(&id);
    }

    pub fn retain(&mut self, live: &[PaneId]) {
        self.states.retain(|id, _| live.contains(id));
    }

    pub fn update(&mut self, id: PaneId, message: PaneMessage) {
        match (self.get_mut(id), message) {
            (Some(PaneState::Queue(state)), PaneMessage::Queue(message)) => {
                queue::update(state, &message);
            }
            (Some(PaneState::Timeline(state)), PaneMessage::Timeline(message)) => {
                timeline::update(state, &message);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_appears_exactly_once_in_all() {
        for kind in PaneKind::ALL {
            let count = PaneKind::ALL.iter().filter(|k| **k == kind).count();
            assert_eq!(count, 1, "{kind:?} listed {count} times in ALL");
        }
    }

    #[test]
    fn categories_are_contiguous_in_all() {
        let groups = PaneKind::by_category();
        let mut seen: Vec<PaneCategory> = Vec::new();
        for (category, _) in &groups {
            assert!(
                !seen.contains(category),
                "{category:?} appears in two separate groups"
            );
            seen.push(*category);
        }
    }

    #[test]
    fn by_category_covers_all_kinds() {
        let grouped: Vec<PaneKind> = PaneKind::by_category()
            .into_iter()
            .flat_map(|(_, kinds)| kinds)
            .collect();
        assert_eq!(grouped, PaneKind::ALL.to_vec());
    }

    #[test]
    fn empty_query_matches_everything() {
        assert_eq!(PaneKind::search("").len(), PaneKind::ALL.len());
    }

    #[test]
    fn search_matches_title_case_insensitively() {
        assert_eq!(PaneKind::search("NOW PLAY"), vec![PaneKind::NowPlaying]);
    }

    #[test]
    fn search_matches_keywords() {
        assert!(PaneKind::search("metadata").contains(&PaneKind::TrackInfo));
        assert!(PaneKind::search("spectrum").contains(&PaneKind::Visualiser));
        assert!(PaneKind::search("equalizer").contains(&PaneKind::Equaliser));
    }

    #[test]
    fn search_matches_category_name() {
        let browse = PaneKind::search("browse");
        assert!(browse.contains(&PaneKind::Albums));
        assert!(!browse.contains(&PaneKind::Lyrics));
    }

    #[test]
    fn search_returns_nothing_for_nonsense() {
        assert!(PaneKind::search("zzzznotathing").is_empty());
    }

    #[test]
    fn search_preserves_all_ordering() {
        let results = PaneKind::search("s");
        let mut expected = results.clone();
        expected.sort_by_key(|kind| {
            PaneKind::ALL
                .iter()
                .position(|k| k == kind)
                .expect("kind is in ALL")
        });
        assert_eq!(results, expected);
    }
}
