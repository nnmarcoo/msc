//! Explore state: what has been searched for online, and what is downloading.
//!
//! This is [`crate::browsing`]'s counterpart for music that is not in the
//! library yet, and it sits on the app for the same reason: a remote query
//! describes what the user is looking for, not how one pane draws, so two
//! Explore panes show the same results. Only per-pane drawing state would
//! belong in [`crate::pane::PaneState`], and this has none.
//!
//! A search is issued per keystroke after a debounce, which means several are
//! usually in flight at once and they can land out of order — a short query
//! returning slower than the longer one typed after it is routine, since the
//! response size differs. [`Generation`] is what makes that safe: every query
//! change bumps a counter, a reply carries the counter it was issued under, and
//! one that no longer matches is dropped. Without it a stale reply overwrites a
//! newer one and the pane shows results for a prefix of what the field says,
//! which looks like the search is simply wrong.
//!
//! The counter is checked rather than the query text compared, because the same
//! text can be current, then stale, then current again as the user deletes and
//! retypes, and a text comparison would accept a reply issued under the first of
//! those for the third. A counter only ever moves forward.
//!
//! A search in flight does *not* clear what is already on screen. Typing nine
//! characters issues nine searches, and blanking the body for each one meant the
//! grid the user was reading was destroyed and rebuilt nine times — the pane
//! never held still long enough to scan. [`Stage::Searching`] is therefore only
//! entered when there is nothing to keep, and [`Explore::is_searching`] is what
//! the pane shows beside the field the rest of the time. A reply still replaces
//! the results wholesale, so an empty one clears them; what is preserved is the
//! *previous* answer while the next is being fetched, not a stale one after it.
//!
//! `pending` is deliberately not a cancellation: iced tasks cannot be cancelled
//! once spawned, so a superseded search still runs to completion and still costs
//! its request. What it cannot do is change the screen. That is the whole
//! guarantee, and it is enough — the request is cheap and the cache absorbs the
//! repeat.
//!
//! [`Held`] is the set of results the library already owns, answered by
//! [`verse_core::explore::already_held`] once per settled listing rather than
//! per row per frame. That check folds every title in the library to compare it,
//! so running it from `view` cost a full library scan and two allocations per
//! track for every row drawn, sixty times a second — the same cost
//! [`crate::app`]'s `visible_ids` exists to avoid, for the same reason. It is
//! refreshed wherever the listing changes or the library does, and nowhere
//! else; those are its only two inputs.
//!
//! [`Downloads`] is keyed by the recording's id rather than by row, because the
//! row a download started from moves the moment the query changes, and a
//! progress bar must follow the recording rather than the position.
//!
//! A finished download does not enter the library where it lands. Ingesting one
//! file reloads the whole library — every row, both indexes, every album — at a
//! cost set by the size of the collection, and an album settling track by track
//! paid that per track while the user watched. So a finished file is parked in
//! [`Downloads::landed`] and the app drains the lot on a timer through
//! [`verse_core::Library::ingest_many`], which pays the reload once.
//!
//! The delay that costs is bounded by the drain interval and is invisible
//! against a download measured in seconds. What it must not do is read as
//! unfinished: a parked row shows as complete immediately, since the bytes are
//! on disk and the waiting is genuinely over — only the track id it will carry
//! is still unknown.
//!
//! [`Download`] and the readers on [`Stage`] are in place before the pane that
//! draws them, matching [`crate::pane`]: the state and its rules are worth
//! settling on their own, and wiring the view to them later is then a
//! self-contained change rather than a redesign.

use std::collections::{HashMap, HashSet};

use verse_core::explore::{Found, FoundAlbum};

pub const SEARCH_LIMIT: usize = 20;

pub const SIMILAR_LIMIT: usize = 25;

pub const ALBUM_LIMIT: usize = 12;

pub const BROWSE_LIMIT: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Generation(u64);

impl Generation {
    fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Results {
    pub albums: Vec<FoundAlbum>,
    pub tracks: Vec<Found>,
}

impl Results {
    pub fn is_empty(&self) -> bool {
        self.albums.is_empty() && self.tracks.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Opened {
    Loading(String),
    Ready(Box<FoundAlbum>),
    Failed(String, String),
}

impl Opened {
    pub fn id(&self) -> &str {
        match self {
            Opened::Loading(id) | Opened::Failed(id, _) => id,
            Opened::Ready(album) => &album.id,
        }
    }

    pub fn album(&self) -> Option<&FoundAlbum> {
        match self {
            Opened::Ready(album) => Some(album),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Shelf {
    pub label: String,
    pub albums: Vec<FoundAlbum>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    Idle,
    Browse(Vec<Shelf>),
    Searching,
    Results(Box<Results>),
    Similar(String, Vec<Found>),
    Failed(String),
}

impl Stage {
    pub fn tracks(&self) -> &[Found] {
        match self {
            Stage::Results(results) => &results.tracks,
            Stage::Similar(_, found) => found,
            Stage::Idle | Stage::Browse(_) | Stage::Searching | Stage::Failed(_) => &[],
        }
    }

    #[cfg(test)]
    pub fn albums(&self) -> impl Iterator<Item = &FoundAlbum> {
        let (listed, shelved) = match self {
            Stage::Results(results) => (results.albums.as_slice(), [].as_slice()),
            Stage::Browse(shelves) => ([].as_slice(), shelves.as_slice()),
            _ => ([].as_slice(), [].as_slice()),
        };

        listed
            .iter()
            .chain(shelved.iter().flat_map(|shelf| shelf.albums.iter()))
    }

    #[cfg(test)]
    pub fn is_busy(&self) -> bool {
        matches!(self, Stage::Searching)
    }

    pub fn holds_results(&self) -> bool {
        match self {
            Stage::Results(results) => !results.is_empty(),
            Stage::Browse(shelves) => shelves.iter().any(|shelf| !shelf.albums.is_empty()),
            Stage::Similar(_, found) => !found.is_empty(),
            Stage::Idle | Stage::Searching | Stage::Failed(_) => false,
        }
    }

    pub fn can_go_back(&self) -> bool {
        matches!(self, Stage::Similar(..))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Download {
    Queued,
    Running(f32),
    Done(i64),
    Failed(String),
}

impl Download {
    pub fn is_settled(&self) -> bool {
        matches!(self, Download::Done(_) | Download::Failed(_))
    }
}

#[derive(Debug, Default)]
pub struct Downloads {
    by_id: HashMap<String, Download>,
    landed: Vec<(String, std::path::PathBuf)>,
}

impl Downloads {
    pub fn get(&self, id: &str) -> Option<&Download> {
        self.by_id.get(id)
    }

    pub fn set(&mut self, id: &str, state: Download) {
        self.by_id.insert(id.to_owned(), state);
    }

    pub fn landed(&mut self, id: &str, path: std::path::PathBuf) {
        self.set(id, Download::Running(1.0));
        self.landed.push((id.to_owned(), path));
    }

    pub fn waiting(&self) -> bool {
        !self.landed.is_empty()
    }

    pub fn take_landed(&mut self) -> Vec<(String, std::path::PathBuf)> {
        std::mem::take(&mut self.landed)
    }

    #[cfg(test)]
    pub fn running(&self) -> usize {
        self.by_id
            .values()
            .filter(|state| !state.is_settled())
            .count()
    }

    #[cfg(test)]
    pub fn holds(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    pub fn forget_settled(&mut self) {
        self.by_id.retain(|_, state| !state.is_settled());
    }
}

#[derive(Debug, Default)]
pub struct Held {
    ids: HashSet<String>,
}

impl Held {
    pub fn holds(&self, id: &str) -> bool {
        self.ids.contains(id)
    }

    pub fn rebuild<'a>(
        &mut self,
        rows: impl Iterator<Item = &'a Found>,
        mut owned: impl FnMut(&Found) -> bool,
    ) {
        self.ids.clear();

        for row in rows {
            if owned(row) {
                self.ids.insert(row.id.clone());
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fetcher {
    #[default]
    Unknown,
    Ready,
    Missing,
}

impl Fetcher {
    pub fn can_download(self) -> bool {
        matches!(self, Fetcher::Ready | Fetcher::Unknown)
    }
}

#[derive(Debug)]
pub struct Explore {
    pub query: String,
    pub stage: Stage,
    pub opened: Option<Opened>,
    pub downloads: Downloads,
    pub held: Held,
    pub fetcher: Fetcher,
    generation: Generation,
    pending: Option<Generation>,
}

impl Default for Explore {
    fn default() -> Self {
        Self {
            query: String::new(),
            stage: Stage::Idle,
            opened: None,
            downloads: Downloads::default(),
            held: Held::default(),
            fetcher: Fetcher::default(),
            generation: Generation::default(),
            pending: None,
        }
    }
}

impl Explore {
    pub fn query_changed(&mut self, query: String) -> Option<(Generation, String)> {
        self.query = query;
        self.generation = self.generation.next();

        let trimmed = self.query.trim().to_owned();
        if trimmed.is_empty() {
            self.stage = Stage::Idle;
            self.pending = None;
            return None;
        }

        self.begin_keeping_results();
        Some((self.generation, trimmed))
    }

    fn begin_keeping_results(&mut self) {
        if !self.stage.holds_results() {
            self.stage = Stage::Searching;
        }

        self.pending = Some(self.generation);
    }

    pub fn open(&mut self, id: &str) -> bool {
        if self.opened.as_ref().is_some_and(|open| open.id() == id) {
            self.opened = None;
            return false;
        }

        self.opened = Some(Opened::Loading(id.to_owned()));
        true
    }

    pub fn opened_settled(&mut self, id: &str, album: Result<FoundAlbum, String>) {
        if self.opened.as_ref().is_none_or(|open| open.id() != id) {
            return;
        }

        self.opened = Some(match album {
            Ok(album) => Opened::Ready(Box::new(album)),
            Err(reason) => Opened::Failed(id.to_owned(), reason),
        });
    }

    #[cfg(test)]
    pub fn is_open(&self, id: &str) -> bool {
        self.opened.as_ref().is_some_and(|open| open.id() == id)
    }

    pub fn begin(&mut self) -> Generation {
        self.generation = self.generation.next();
        self.begin_keeping_results();
        self.generation
    }

    pub fn is_searching(&self) -> bool {
        self.pending.is_some()
    }

    pub fn is_current(&self, generation: Generation) -> bool {
        self.pending == Some(generation)
    }

    pub fn settle(&mut self, generation: Generation, stage: Stage) -> bool {
        if !self.is_current(generation) {
            return false;
        }

        self.stage = stage;
        self.pending = None;
        self.opened = None;
        self.downloads.forget_settled();
        true
    }

    pub fn drawable(&self) -> impl Iterator<Item = &Found> {
        self.stage.tracks().iter().chain(
            self.opened
                .as_ref()
                .and_then(Opened::album)
                .map_or(&[][..], |album| album.tracks.as_slice()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(id: &str) -> Found {
        Found {
            id: id.to_owned(),
            title: format!("Track {id}"),
            artist: Some("Someone".to_owned()),
            album: None,
            album_id: None,
            duration: Some(100),
            cover_url: None,
            explicit: false,
        }
    }

    fn results(tracks: Vec<Found>) -> Stage {
        Stage::Results(Box::new(Results {
            albums: Vec::new(),
            tracks,
        }))
    }

    #[test]
    fn typing_a_query_asks_for_a_search() {
        let mut explore = Explore::default();
        let issued = explore.query_changed("radiohead".to_owned());

        let (generation, query) = issued.expect("a search is issued");
        assert_eq!(query, "radiohead");
        assert!(explore.is_current(generation));
        assert!(explore.stage.is_busy());
    }

    #[test]
    fn a_reply_fills_the_pane() {
        let mut explore = Explore::default();
        let (generation, _) = explore.query_changed("a".to_owned()).expect("issued");

        assert!(explore.settle(generation, results(vec![found("x")])));
        assert_eq!(explore.stage.tracks().len(), 1);
        assert!(!explore.stage.is_busy());
    }

    #[test]
    fn typing_again_keeps_the_results_already_on_screen() {
        let mut explore = Explore::default();
        let (first, _) = explore.query_changed("rad".to_owned()).expect("issued");
        explore.settle(first, results(vec![found("showing")]));

        explore
            .query_changed("radiohead".to_owned())
            .expect("issued");

        assert_eq!(
            explore.stage.tracks().len(),
            1,
            "the grid the user was reading was thrown away mid-keystroke"
        );
        assert!(explore.is_searching(), "the new search is still in flight");
    }

    #[test]
    fn a_first_search_with_nothing_to_keep_shows_that_it_is_working() {
        let mut explore = Explore::default();
        explore
            .query_changed("radiohead".to_owned())
            .expect("issued");

        assert_eq!(
            explore.stage,
            Stage::Searching,
            "a cold search has no results to hold, so the body says so"
        );
    }

    #[test]
    fn a_search_that_finds_nothing_stops_showing_the_previous_results() {
        let mut explore = Explore::default();
        let (first, _) = explore.query_changed("rad".to_owned()).expect("issued");
        explore.settle(first, results(vec![found("old")]));

        let (second, _) = explore.query_changed("zzzz".to_owned()).expect("issued");
        explore.settle(second, results(Vec::new()));

        assert!(
            explore.stage.tracks().is_empty(),
            "stale results outlived the search that replaced them"
        );
    }

    #[test]
    fn a_stale_reply_is_dropped() {
        let mut explore = Explore::default();
        let (first, _) = explore.query_changed("rad".to_owned()).expect("issued");
        let (second, _) = explore
            .query_changed("radiohead".to_owned())
            .expect("issued");

        assert!(!explore.settle(first, results(vec![found("stale")])));
        assert!(explore.stage.is_busy(), "the newer search is still running");

        assert!(explore.settle(second, results(vec![found("fresh")])));
        assert_eq!(explore.stage.tracks()[0].id, "fresh");
    }

    #[test]
    fn retyping_the_same_text_does_not_accept_the_older_reply() {
        let mut explore = Explore::default();
        let (first, _) = explore.query_changed("nine".to_owned()).expect("issued");
        explore.query_changed(String::new());
        let (third, _) = explore.query_changed("nine".to_owned()).expect("issued");

        assert_ne!(first, third, "the same text is a different generation");
        assert!(!explore.settle(first, results(vec![found("old")])));
        assert!(explore.settle(third, results(vec![found("new")])));
    }

    #[test]
    fn clearing_the_query_asks_for_nothing_and_shows_nothing() {
        let mut explore = Explore::default();
        explore.query_changed("radiohead".to_owned());

        assert!(explore.query_changed(String::new()).is_none());
        assert_eq!(explore.stage, Stage::Idle);
    }

    #[test]
    fn a_query_of_only_spaces_is_not_a_search() {
        let mut explore = Explore::default();
        assert!(explore.query_changed("   ".to_owned()).is_none());
        assert_eq!(explore.stage, Stage::Idle);
    }

    #[test]
    fn a_reply_that_lands_after_clearing_is_dropped() {
        let mut explore = Explore::default();
        let (generation, _) = explore.query_changed("a".to_owned()).expect("issued");
        explore.query_changed(String::new());

        assert!(!explore.settle(generation, results(vec![found("late")])));
        assert_eq!(explore.stage, Stage::Idle);
    }

    #[test]
    fn a_failure_is_reported_rather_than_left_spinning() {
        let mut explore = Explore::default();
        let (generation, _) = explore.query_changed("a".to_owned()).expect("issued");

        explore.settle(generation, Stage::Failed("offline".to_owned()));

        assert!(!explore.stage.is_busy());
        assert!(matches!(explore.stage, Stage::Failed(_)));
    }

    #[test]
    fn opening_an_album_leaves_the_listing_in_place() {
        let mut explore = Explore::default();
        let (search, _) = explore.query_changed("a".to_owned()).expect("issued");
        explore.settle(search, results(vec![found("t")]));

        assert!(explore.open("MPRE1"));
        assert!(explore.is_open("MPRE1"));
        assert_eq!(
            explore.stage.tracks().len(),
            1,
            "the grid the album was opened from must stay on screen"
        );
    }

    #[test]
    fn opening_the_same_album_again_closes_it() {
        let mut explore = Explore::default();

        assert!(explore.open("MPRE1"));
        assert!(!explore.open("MPRE1"), "a second click closes the panel");
        assert!(!explore.is_open("MPRE1"));
    }

    #[test]
    fn a_new_listing_closes_whatever_was_open() {
        let mut explore = Explore::default();
        explore.open("MPRE1");

        let (generation, _) = explore.query_changed("b".to_owned()).expect("issued");
        explore.settle(generation, results(vec![found("x")]));

        assert!(!explore.is_open("MPRE1"));
    }

    #[test]
    fn an_album_that_arrives_after_being_closed_is_dropped() {
        let mut explore = Explore::default();
        explore.open("MPRE1");
        explore.open("MPRE1");

        explore.opened_settled(
            "MPRE1",
            Ok(FoundAlbum {
                release: verse_core::explore::Release::default(),
                id: "MPRE1".to_owned(),
                title: "Album".to_owned(),
                artist: None,
                year: None,
                cover_url: None,
                explicit: false,
                tracks: vec![found("t")],
            }),
        );

        assert!(explore.opened.is_none());
    }

    #[test]
    fn a_download_is_tracked_by_recording_rather_than_by_row() {
        let mut downloads = Downloads::default();
        downloads.set("abc", Download::Queued);
        downloads.set("abc", Download::Running(0.5));

        assert_eq!(downloads.get("abc"), Some(&Download::Running(0.5)));
        assert_eq!(downloads.running(), 1);
        assert!(downloads.holds("abc"));
    }

    #[test]
    fn a_settled_download_no_longer_counts_as_running() {
        let mut downloads = Downloads::default();
        downloads.set("a", Download::Running(0.2));
        downloads.set("b", Download::Done(7));
        downloads.set("c", Download::Failed("nope".to_owned()));

        assert_eq!(downloads.running(), 1);
    }

    #[test]
    fn forgetting_settled_downloads_keeps_the_running_ones() {
        let mut downloads = Downloads::default();
        downloads.set("a", Download::Running(0.2));
        downloads.set("b", Download::Done(7));

        downloads.forget_settled();

        assert!(downloads.holds("a"));
        assert!(!downloads.holds("b"));
    }

    #[test]
    fn a_finished_download_reads_as_complete_before_it_reaches_the_library() {
        let mut downloads = Downloads::default();
        downloads.landed("a", std::path::PathBuf::from("/music/a.m4a"));

        assert_eq!(downloads.get("a"), Some(&Download::Running(1.0)));
        assert!(downloads.waiting());
    }

    #[test]
    fn draining_hands_over_every_landed_file_once() {
        let mut downloads = Downloads::default();
        downloads.landed("a", std::path::PathBuf::from("/music/a.m4a"));
        downloads.landed("b", std::path::PathBuf::from("/music/b.m4a"));

        let drained = downloads.take_landed();

        assert_eq!(drained.len(), 2, "an album's tracks drain together");
        assert!(!downloads.waiting(), "a drained batch is not drained twice");
        assert!(downloads.take_landed().is_empty());
    }

    #[test]
    fn nothing_waiting_is_nothing_to_drain() {
        let mut downloads = Downloads::default();
        downloads.set("a", Download::Running(0.4));

        assert!(!downloads.waiting());
        assert!(downloads.take_landed().is_empty());
    }

    #[test]
    fn a_new_listing_forgets_downloads_that_have_finished() {
        let mut explore = Explore::default();
        let (generation, _) = explore.query_changed("a".to_owned()).expect("issued");

        explore.downloads.set("done", Download::Done(7));
        explore.downloads.set("live", Download::Running(0.3));

        assert!(explore.settle(generation, results(vec![found("x")])));

        assert!(!explore.downloads.holds("done"), "a settled row was kept");
        assert!(
            explore.downloads.holds("live"),
            "a download still running was forgotten mid-flight"
        );
    }

    #[test]
    fn a_generation_keeps_moving_even_at_the_end_of_its_range() {
        let last = Generation(u64::MAX);
        assert_ne!(last.next(), last);
    }

    #[test]
    fn a_burst_of_keystrokes_leaves_only_the_last_showing() {
        let mut explore = Explore::default();

        let issued: Vec<Generation> = ["r", "ra", "rad", "radi", "radiohead"]
            .into_iter()
            .filter_map(|prefix| {
                explore
                    .query_changed(prefix.to_owned())
                    .map(|(generation, _)| generation)
            })
            .collect();

        assert_eq!(issued.len(), 5, "each keystroke asks");

        let (last, superseded) = issued.split_last().expect("five generations");

        for (index, stale) in superseded.iter().enumerate() {
            assert!(
                !explore.settle(*stale, results(vec![found(&index.to_string())])),
                "reply {index} is stale and must not show"
            );
        }

        assert!(explore.settle(*last, results(vec![found("radiohead")])));
        assert_eq!(explore.stage.tracks()[0].id, "radiohead");
    }

    #[test]
    fn replies_arriving_backwards_still_leave_the_newest_showing() {
        let mut explore = Explore::default();
        let (first, _) = explore.query_changed("a".to_owned()).expect("issued");
        let (second, _) = explore.query_changed("ab".to_owned()).expect("issued");

        assert!(explore.settle(second, results(vec![found("newest")])));
        assert!(
            !explore.settle(first, results(vec![found("oldest")])),
            "the older reply arrives last and must still be refused"
        );

        assert_eq!(explore.stage.tracks()[0].id, "newest");
    }
}
