//! What a pane can contain.
//!
//! [`PaneKind`] is an enum rather than a trait object, and per-pane state lives
//! in [`PaneState`] keyed by [`PaneId`]. Reaching one pane's state is a match,
//! not a downcast, so adding a kind fails to compile everywhere that must
//! handle it instead of silently doing nothing at runtime.

pub mod library;
pub mod queue;
pub mod view;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::layout::PaneId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneKind {
    Library,
    Queue,
    NowPlaying,
    Empty,
}

impl PaneKind {
    pub const ALL: [PaneKind; 4] = [
        PaneKind::Library,
        PaneKind::Queue,
        PaneKind::NowPlaying,
        PaneKind::Empty,
    ];

    pub fn title(self) -> &'static str {
        match self {
            PaneKind::Library => "Library",
            PaneKind::Queue => "Queue",
            PaneKind::NowPlaying => "Now Playing",
            PaneKind::Empty => "Empty",
        }
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
            PaneKind::NowPlaying | PaneKind::Empty => Self::Stateless,
        }
    }
}

/// Per-pane state, kept beside the layout rather than inside it so that the
/// layout stays serialisable and free of runtime-only data.
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
                queue::update(state, message);
            }
            _ => {}
        }
    }
}
