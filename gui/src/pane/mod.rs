//! What a pane can contain.
//!
//! [`PaneKind`] is an enum rather than a trait object, and per-pane state lives
//! in [`PaneState`] keyed by [`PaneId`]. Reaching one pane's state is a match,
//! not a downcast, so adding a kind fails to compile everywhere that must
//! handle it instead of silently doing nothing at runtime.
//!
//! The per-kind state, messages, and dispatch are in place but not yet driven:
//! panes currently render as labels while the layout mechanics are the focus.
//! The scaffolding stays so that wiring real content later is a self-contained
//! change rather than a rework.
#![allow(dead_code)]

pub mod controls;
pub mod library;
pub mod queue;
pub mod view;

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
    Albums,
    Artists,
    Playlists,
    Folders,
    Queue,
    NowPlaying,
    Controls,
    History,
    Lyrics,
    TrackInfo,
    Visualiser,
    Equaliser,
    Settings,
    Empty,
}

impl PaneKind {
    pub const ALL: [PaneKind; 15] = [
        PaneKind::Library,
        PaneKind::Albums,
        PaneKind::Artists,
        PaneKind::Playlists,
        PaneKind::Folders,
        PaneKind::Queue,
        PaneKind::NowPlaying,
        PaneKind::Controls,
        PaneKind::History,
        PaneKind::Lyrics,
        PaneKind::TrackInfo,
        PaneKind::Visualiser,
        PaneKind::Equaliser,
        PaneKind::Settings,
        PaneKind::Empty,
    ];

    pub fn title(self) -> &'static str {
        match self {
            PaneKind::Library => "Library",
            PaneKind::Albums => "Albums",
            PaneKind::Artists => "Artists",
            PaneKind::Playlists => "Playlists",
            PaneKind::Folders => "Folders",
            PaneKind::Queue => "Queue",
            PaneKind::NowPlaying => "Now Playing",
            PaneKind::Controls => "Controls",
            PaneKind::History => "History",
            PaneKind::Lyrics => "Lyrics",
            PaneKind::TrackInfo => "Track Info",
            PaneKind::Visualiser => "Visualiser",
            PaneKind::Equaliser => "Equaliser",
            PaneKind::Settings => "Settings",
            PaneKind::Empty => "Empty",
        }
    }

    pub fn category(self) -> PaneCategory {
        match self {
            PaneKind::Library
            | PaneKind::Albums
            | PaneKind::Artists
            | PaneKind::Playlists
            | PaneKind::Folders => PaneCategory::Browse,
            PaneKind::Queue | PaneKind::NowPlaying | PaneKind::Controls | PaneKind::History => {
                PaneCategory::Playback
            }
            PaneKind::Lyrics | PaneKind::TrackInfo | PaneKind::Visualiser => PaneCategory::Detail,
            PaneKind::Equaliser | PaneKind::Settings | PaneKind::Empty => PaneCategory::Tools,
        }
    }

    pub fn keywords(self) -> &'static str {
        match self {
            PaneKind::Library => "songs tracks music collection",
            PaneKind::Albums => "records releases discography",
            PaneKind::Artists => "bands performers musicians",
            PaneKind::Playlists => "mixes sets collections",
            PaneKind::Folders => "files directories browse disk",
            PaneKind::Queue => "up next playlist upcoming",
            PaneKind::NowPlaying => "current track player",
            PaneKind::Controls => "transport play pause next previous skip",
            PaneKind::History => "recent played log past",
            PaneKind::Lyrics => "words text karaoke",
            PaneKind::TrackInfo => "metadata tags details properties",
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
    Library(library::Message),
    Queue(queue::Message),
}

#[derive(Debug)]
pub enum PaneState {
    Library(library::State),
    Queue(queue::State),
    Stateless,
}

impl PaneState {
    fn for_kind(kind: PaneKind) -> Self {
        match kind {
            PaneKind::Library => Self::Library(library::State::default()),
            PaneKind::Queue => Self::Queue(queue::State::default()),
            PaneKind::Albums
            | PaneKind::Artists
            | PaneKind::Playlists
            | PaneKind::Folders
            | PaneKind::NowPlaying
            | PaneKind::Controls
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
            (Some(PaneState::Library(state)), PaneMessage::Library(message)) => {
                library::update(state, message);
            }
            (Some(PaneState::Queue(state)), PaneMessage::Queue(message)) => {
                queue::update(state, &message);
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
