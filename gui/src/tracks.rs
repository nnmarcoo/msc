//! Track state shared by every pane that shows tracks.
//!
//! Search, selection, and hover are properties of a *track*, not of the pane
//! that happens to display it, so they live on the app and reach panes through
//! [`Context`] rather than through per-pane state. Hovering a row in the library
//! therefore lights the same track up in the queue without the two panes knowing
//! about each other: neither reads the other's state, both read this.
//!
//! That split is the rule for where new state goes. Anything keyed on a track id
//! belongs here; anything describing how one pane draws, such as its scroll offset
//! or column widths, belongs in that pane's own state, since two panes of the same
//! kind must be able to disagree about it.
//!
//! [`Selection`] holds track ids rather than row indices because the rows a pane
//! shows are a filtered, ordered view that changes under it. An index-keyed
//! selection would survive a search keystroke as a *different* set of tracks
//! rather than the same ones, silently. Ids also let a selection made in one
//! pane mean the same thing in a pane listing the tracks in another order.
//!
//! The anchor is an index, since a shift-range is a fact about the list on
//! screen and not about the library. It is recorded alongside the id it pointed
//! at so a range extended after the list changed can tell that its pivot moved,
//! and fall back to a plain click rather than selecting an arbitrary span.
//! `live_anchor` is that check, and `extend_to_ids` degrades to `select` when it
//! fails.
//!
//! [`Context`] is built once per frame in the app's view and handed down; panes
//! read it and never own it. `visible` drops missing files, since a pane lists
//! what can actually be played, and [`RowState`] carries the three flags a row
//! draws from. They are independent rather than ranked, because a track can be
//! playing, selected, and hovered at once and the widget layers them.
//!
//! `visible` re-filters the library every frame rather than caching the rows,
//! since a row index only means anything against the list the click was made
//! on. That makes the filter itself the thing that has to be cheap, so
//! `contains_fold` allocates nothing for the ASCII tags that are almost all of
//! them: it compares case-insensitively in place. Lowercasing every field of
//! every track instead, as this first did, allocated five strings per track per
//! frame and cost more than the frame budget on a large library, where the search
//! alone was slower than drawing. Non-ASCII text still falls back to a real
//! `to_lowercase`, so a query matches `Björk` the way it always did; only the
//! rows that need Unicode folding pay for it.
//!
//! `retain_listed` is only ever called with the *unfiltered* library, never with
//! the rows a search left on screen: narrowing a query would otherwise discard a
//! selection the user made before typing. A selection therefore survives a
//! search and reappears when it is cleared. `ordered_ids` returns the selection
//! in display order, so acting on it queues tracks the way the user sees them
//! rather than by id.
//!
//! `toggle` makes the row it flipped the new pivot whether it added or removed
//! it, matching the file managers this borrows from. The right-click rule lives
//! in the app rather than here: a click inside the selection keeps it, one
//! outside replaces it, so a menu never acts on rows that are out of sight.
//!
//! `queued` flattens the queue's three parts into one list of [`QueueRow`]s,
//! each tagged with the [`Slot`] it came from, so a pane draws played, current,
//! and upcoming tracks in play order without reaching into the queue itself.
//! History is included only when asked for, since it grows without bound.
//!
//! `upcoming` is a row's index into the queue's upcoming deque, and `None` for
//! the rows that have no place in it, history and the current track. It exists
//! because a row's position in the flattened list is *not* the index the queue
//! removes by: `Queue::remove` indexes the upcoming deque alone, so passing it a
//! flat index deleted whatever sat that far into the queue instead of the row
//! that was clicked, off by the current track and by the whole of history when
//! it was shown. The index is taken before missing tracks are filtered out, so
//! a queued file that has left the library shifts nothing.
//!
//! Carrying the index rather than recomputing it in the pane is what makes the
//! wrong index unrepresentable: a row that cannot be removed has nothing to
//! remove by, so the pane has no flat index available to pass by mistake.
//!
//! A queue may hold the same track twice, so its rows are positional while
//! `row_state` stays keyed on the id. Hovering a track therefore lights *every*
//! copy of it in the queue, which is the truth about where that track sits; a
//! positional highlight would claim the other copies are a different song. This
//! is also why [`QueueRow`] carries the track rather than an index: position
//! identifies a row, the id identifies the music.
//!
//! `hovered` can be a bare track id only because both lists are single widgets
//! that know which row the cursor is over. Per-row widgets need more, since their
//! arrivals and departures race in layout order and an id cannot tell "the pointer
//! left me" from "the pointer moved to another copy of me". See
//! [`crate::widgets::queue_list`].

use std::collections::BTreeSet;

use verse_core::{Library, Queue, Track};

#[derive(Clone, Copy)]
pub struct QueueRow<'a> {
    pub track: &'a Track,
    pub slot: Slot,
    pub upcoming: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Played,
    Current,
    Upcoming,
}

#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub library: &'a Library,
    pub queue: &'a Queue,
    pub search: &'a str,
    pub selection: &'a Selection,
    pub hovered: Option<i64>,
    pub playing: Option<i64>,
}

impl<'a> Context<'a> {
    pub fn visible(&self) -> Vec<&'a Track> {
        let query = self.search.trim().to_lowercase();
        self.library
            .available()
            .filter(|track| matches(track, &query))
            .collect()
    }

    pub fn queued(&self, show_history: bool) -> Vec<QueueRow<'a>> {
        let mut rows = Vec::with_capacity(self.queue.len());

        if show_history {
            rows.extend(self.rows_of(self.queue.history(), Slot::Played));
        }
        if let Some(track) = self.queue.current().and_then(|id| self.library.track(id)) {
            rows.push(QueueRow {
                track,
                slot: Slot::Current,
                upcoming: None,
            });
        }
        rows.extend(self.rows_of(self.queue.upcoming(), Slot::Upcoming));

        rows
    }

    fn rows_of(
        &self,
        ids: &'a std::collections::VecDeque<i64>,
        slot: Slot,
    ) -> impl Iterator<Item = QueueRow<'a>> {
        let library = self.library;
        ids.iter()
            .enumerate()
            .filter_map(move |(position, &id)| Some((position, library.track(id)?)))
            .map(move |(position, track)| QueueRow {
                track,
                slot,
                upcoming: (slot == Slot::Upcoming).then_some(position),
            })
    }

    pub fn row_state(&self, id: i64) -> RowState {
        row_state(id, self.selection, self.hovered, self.playing)
    }
}

fn row_state(
    id: i64,
    selection: &Selection,
    hovered: Option<i64>,
    playing: Option<i64>,
) -> RowState {
    RowState {
        selected: selection.contains(id),
        hovered: hovered == Some(id),
        playing: playing == Some(id),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RowState {
    pub selected: bool,
    pub hovered: bool,
    pub playing: bool,
}

fn matches(track: &Track, query_lower: &str) -> bool {
    if query_lower.is_empty() {
        return true;
    }
    let field = |value: Option<&str>| value.is_some_and(|value| contains_fold(value, query_lower));
    field(track.title())
        || field(track.track_artist())
        || field(track.album())
        || field(track.album_artist())
        || field(track.genre())
}

fn contains_fold(haystack: &str, needle_lower: &str) -> bool {
    if haystack.is_ascii() && needle_lower.is_ascii() {
        return contains_ascii_fold(haystack, needle_lower);
    }
    haystack.to_lowercase().contains(needle_lower)
}

fn contains_ascii_fold(haystack: &str, needle_lower: &str) -> bool {
    let (haystack, needle) = (haystack.as_bytes(), needle_lower.as_bytes());
    needle.len() <= haystack.len()
        && haystack
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle))
}

#[derive(Debug, Default, Clone)]
pub struct Selection {
    selected: BTreeSet<i64>,
    anchor: Option<Anchor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Anchor {
    index: usize,
    id: i64,
}

impl Selection {
    pub fn contains(&self, id: i64) -> bool {
        self.selected.contains(&id)
    }

    pub fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }

    pub fn len(&self) -> usize {
        self.selected.len()
    }

    pub fn ordered_ids(&self, ids: &[i64]) -> Vec<i64> {
        ids.iter()
            .copied()
            .filter(|id| self.selected.contains(id))
            .collect()
    }

    pub fn clear(&mut self) {
        self.selected.clear();
        self.anchor = None;
    }

    pub fn select(&mut self, index: usize, id: i64) {
        self.selected.clear();
        self.selected.insert(id);
        self.anchor = Some(Anchor { index, id });
    }

    pub fn toggle(&mut self, index: usize, id: i64) {
        if !self.selected.remove(&id) {
            self.selected.insert(id);
        }
        self.anchor = Some(Anchor { index, id });
    }

    pub fn extend_to_ids(&mut self, index: usize, ids: &[i64]) {
        let Some(&id) = ids.get(index) else {
            return;
        };

        let Some(anchor) = self.live_anchor(ids) else {
            self.select(index, id);
            return;
        };

        let (low, high) = if anchor.index <= index {
            (anchor.index, index)
        } else {
            (index, anchor.index)
        };

        self.selected.clear();
        self.selected
            .extend(ids.get(low..=high).unwrap_or_default().iter().copied());
    }

    pub fn select_all_ids(&mut self, ids: &[i64]) {
        self.selected.clear();
        self.selected.extend(ids.iter().copied());
        self.anchor = None;
    }

    pub fn retain_listed(&mut self, ids: &[i64]) {
        let live: BTreeSet<i64> = ids.iter().copied().collect();
        self.selected.retain(|id| live.contains(id));
        if self.live_anchor(ids).is_none() {
            self.anchor = None;
        }
    }

    fn live_anchor(&self, ids: &[i64]) -> Option<Anchor> {
        let anchor = self.anchor?;
        let still_there = ids.get(anchor.index).is_some_and(|&id| id == anchor.id);
        still_there.then_some(anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(count: i64) -> Vec<i64> {
        (0..count).collect()
    }

    #[test]
    fn a_click_replaces_the_whole_selection() {
        let rows = ids(5);
        let mut selection = Selection::default();

        selection.select_all_ids(&rows);
        selection.select(2, 2);

        assert_eq!(selection.len(), 1);
        assert!(selection.contains(2));
    }

    #[test]
    fn ctrl_click_adds_then_removes_the_same_track() {
        let mut selection = Selection::default();

        selection.toggle(1, 1);
        assert!(selection.contains(1));

        selection.toggle(1, 1);
        assert!(!selection.contains(1));
    }

    #[test]
    fn ctrl_click_leaves_other_tracks_alone() {
        let mut selection = Selection::default();
        selection.select(0, 0);
        selection.toggle(3, 3);

        assert!(selection.contains(0));
        assert!(selection.contains(3));
        assert_eq!(selection.len(), 2);
    }

    #[test]
    fn shift_click_selects_the_span_between_pivot_and_click() {
        let rows = ids(6);
        let mut selection = Selection::default();

        selection.select(1, 1);
        selection.extend_to_ids(4, &rows);

        assert_eq!(selection.ordered_ids(&rows), vec![1, 2, 3, 4]);
    }

    #[test]
    fn shift_click_spans_the_same_range_in_either_direction() {
        let rows = ids(6);

        let mut downward = Selection::default();
        downward.select(1, 1);
        downward.extend_to_ids(4, &rows);

        let mut upward = Selection::default();
        upward.select(4, 4);
        upward.extend_to_ids(1, &rows);

        assert_eq!(downward.ordered_ids(&rows), upward.ordered_ids(&rows));
    }

    #[test]
    fn a_second_shift_click_re_extends_from_the_original_pivot() {
        let rows = ids(8);
        let mut selection = Selection::default();

        selection.select(2, 2);
        selection.extend_to_ids(5, &rows);
        selection.extend_to_ids(3, &rows);

        assert_eq!(
            selection.ordered_ids(&rows),
            vec![2, 3],
            "the pivot moved with the first range instead of staying put"
        );
    }

    #[test]
    fn shift_click_without_a_pivot_acts_like_a_plain_click() {
        let rows = ids(5);
        let mut selection = Selection::default();

        selection.extend_to_ids(3, &rows);

        assert_eq!(selection.ordered_ids(&rows), vec![3]);
    }

    #[test]
    fn a_selection_survives_the_list_being_reordered() {
        let forward = ids(4);
        let mut selection = Selection::default();
        selection.select(0, 0);
        selection.toggle(2, 2);

        let reversed: Vec<i64> = forward.iter().rev().copied().collect();

        assert_eq!(
            selection.ordered_ids(&reversed),
            vec![2, 0],
            "ids should follow the tracks, in the new display order"
        );
    }

    #[test]
    fn ordered_follows_display_order_not_id_order() {
        let reversed: Vec<i64> = ids(4).into_iter().rev().collect();
        let mut selection = Selection::default();
        selection.select_all_ids(&reversed);

        assert_eq!(selection.ordered_ids(&reversed), vec![3, 2, 1, 0]);
    }

    #[test]
    fn a_stale_pivot_falls_back_to_a_plain_click() {
        let mut selection = Selection::default();
        selection.select(4, 4);

        let filtered = ids(2);
        selection.extend_to_ids(1, &filtered);

        assert_eq!(
            selection.ordered_ids(&filtered),
            vec![1],
            "a pivot past the end of the filtered list selected a span anyway"
        );
    }

    #[test]
    fn a_pivot_pointing_at_a_different_track_is_not_used() {
        let mut selection = Selection::default();
        selection.select(1, 1);

        let shifted: Vec<i64> = ids(6).into_iter().skip(3).collect();
        selection.extend_to_ids(2, &shifted);

        assert_eq!(
            selection.ordered_ids(&shifted),
            vec![5],
            "index 1 now holds a different track, so the range was meaningless"
        );
    }

    #[test]
    fn select_all_takes_only_the_visible_rows() {
        let filtered = ids(3);
        let mut selection = Selection::default();

        selection.select_all_ids(&filtered);

        assert_eq!(selection.len(), 3);
    }

    #[test]
    fn retaining_drops_tracks_that_left_the_library() {
        let rows = ids(5);
        let mut selection = Selection::default();
        selection.select_all_ids(&rows);

        let remaining = ids(2);
        selection.retain_listed(&remaining);

        assert_eq!(selection.ordered_ids(&remaining), vec![0, 1]);
        assert_eq!(selection.len(), 2);
    }

    #[test]
    fn retaining_keeps_a_selection_hidden_by_a_search() {
        let rows = ids(5);
        let mut selection = Selection::default();
        selection.select(3, 3);

        selection.retain_listed(&rows);

        assert!(
            selection.contains(3),
            "a track still in the library was dropped"
        );
    }

    fn right_click(selection: &mut Selection, index: usize, ids: &[i64]) {
        if let Some(&id) = ids.get(index)
            && !selection.contains(id)
        {
            selection.select(index, id);
        }
    }

    #[test]
    fn a_query_matches_regardless_of_case() {
        assert!(contains_fold("Blue Monday", "monday"));
        assert!(contains_fold("BLUE MONDAY", "monday"));
        assert!(contains_fold("blue monday", "monday"));
    }

    #[test]
    fn a_query_matches_inside_a_word() {
        assert!(contains_fold("Unfinished Sympathy", "finish"));
        assert!(!contains_fold("Unfinished Sympathy", "zzz"));
    }

    #[test]
    fn an_accented_title_still_folds() {
        assert!(
            contains_fold("BJÖRK", "björk"),
            "a non-ascii field skipped the unicode fallback"
        );
        assert!(contains_fold("Sigur Rós", "rós"));
    }

    #[test]
    fn an_accented_query_does_not_match_the_ascii_letter() {
        assert!(
            !contains_fold("Bjork", "björk"),
            "folding treated o and ö as the same letter"
        );
    }

    #[test]
    fn a_query_longer_than_the_field_matches_nothing() {
        assert!(!contains_fold("Hey", "hey there"));
    }

    #[test]
    fn folding_agrees_with_lowercasing_on_ascii() {
        let fields = ["Blue Monday", "AUTOBAHN", "cassette", "Track 01", ""];
        for field in fields {
            for query in ["monday", "auto", "SETT", "01", "zzz"] {
                let query = query.to_lowercase();
                assert_eq!(
                    contains_fold(field, &query),
                    field.to_lowercase().contains(&query),
                    "{field:?} vs {query:?} disagreed with the lowercasing it replaced"
                );
            }
        }
    }

    /// The flattening [`Context::queued`] performs, over ids alone. `Library`
    /// has no test constructor, so the rows it builds cannot be made here; what
    /// can be checked is the arithmetic that was wrong, which is the mapping
    /// from a row's place on screen to its place in the upcoming deque.
    fn flatten(
        history: &[i64],
        current: Option<i64>,
        upcoming: &[i64],
        show_history: bool,
    ) -> Vec<(i64, Slot, Option<usize>)> {
        let mut rows = Vec::new();
        if show_history {
            rows.extend(history.iter().map(|&id| (id, Slot::Played, None)));
        }
        if let Some(id) = current {
            rows.push((id, Slot::Current, None));
        }
        rows.extend(
            upcoming
                .iter()
                .enumerate()
                .map(|(position, &id)| (id, Slot::Upcoming, Some(position))),
        );
        rows
    }

    #[test]
    fn removing_an_upcoming_row_takes_the_track_that_was_clicked() {
        let upcoming = [10, 11, 12, 13];
        let rows = flatten(&[7, 8], Some(9), &upcoming, false);

        for (id, slot, index) in rows {
            if slot != Slot::Upcoming {
                continue;
            }
            let index = index.expect("an upcoming row carries its queue position");
            assert_eq!(
                upcoming[index], id,
                "row {id} would have removed {} instead",
                upcoming[index]
            );
        }
    }

    #[test]
    fn the_current_track_offsets_no_removal_index() {
        let upcoming = [10, 11, 12];
        let rows = flatten(&[], Some(9), &upcoming, false);

        let first_upcoming = rows
            .iter()
            .find(|(_, slot, _)| *slot == Slot::Upcoming)
            .expect("an upcoming row");

        assert_eq!(
            first_upcoming.2,
            Some(0),
            "the playing row shifted the first upcoming index, so a double-click \
             there removed the track above it"
        );
    }

    #[test]
    fn shown_history_does_not_shift_removal_indices() {
        let upcoming = [10, 11, 12];
        let without = flatten(&[7, 8], Some(9), &upcoming, false);
        let with = flatten(&[7, 8], Some(9), &upcoming, true);

        let indices = |rows: Vec<(i64, Slot, Option<usize>)>| -> Vec<Option<usize>> {
            rows.into_iter()
                .filter(|(_, slot, _)| *slot == Slot::Upcoming)
                .map(|(_, _, index)| index)
                .collect()
        };

        assert_eq!(
            indices(without),
            indices(with),
            "showing history moved the queue positions the rows remove by"
        );
    }

    #[test]
    fn played_and_playing_rows_cannot_be_removed() {
        let rows = flatten(&[7, 8], Some(9), &[10, 11], true);

        for (id, slot, index) in rows {
            match slot {
                Slot::Played | Slot::Current => assert_eq!(
                    index, None,
                    "row {id} ({slot:?}) offered a removal index, so a double-click \
                     would have deleted an unrelated upcoming track"
                ),
                Slot::Upcoming => assert!(index.is_some()),
            }
        }
    }

    #[test]
    fn an_empty_queue_with_history_shown_offers_no_removals() {
        let rows = flatten(&[7, 8], None, &[], true);

        assert!(rows.iter().all(|(_, _, index)| index.is_none()));
    }

    #[test]
    fn every_copy_of_a_hovered_track_lights_up() {
        let selection = Selection::default();
        let queue = [7, 3, 7, 9];

        let lit: Vec<bool> = queue
            .iter()
            .map(|&id| row_state(id, &selection, Some(7), None).hovered)
            .collect();

        assert_eq!(
            lit,
            vec![true, false, true, false],
            "hover is keyed on the track, so both copies of 7 light up"
        );
    }

    #[test]
    fn hovering_a_track_lights_it_up_in_every_pane() {
        let selection = Selection::default();
        assert!(row_state(4, &selection, Some(4), None).hovered);
        assert!(!row_state(5, &selection, Some(4), None).hovered);
    }

    #[test]
    fn a_row_can_be_playing_selected_and_hovered_at_once() {
        let mut selection = Selection::default();
        selection.select(0, 1);

        let state = row_state(1, &selection, Some(1), Some(1));

        assert!(state.selected && state.hovered && state.playing);
    }

    #[test]
    fn the_three_row_flags_are_independent() {
        let selection = Selection::default();
        let state = row_state(1, &selection, None, Some(1));

        assert!(state.playing);
        assert!(!state.hovered, "playing implied hovered");
        assert!(!state.selected, "playing implied selected");
    }

    #[test]
    fn right_clicking_inside_the_selection_keeps_it() {
        let rows = ids(6);
        let mut selection = Selection::default();
        selection.select(1, 1);
        selection.extend_to_ids(3, &rows);

        right_click(&mut selection, 2, &rows);

        assert_eq!(
            selection.ordered_ids(&rows),
            vec![1, 2, 3],
            "a right-click inside the selection collapsed it to one row"
        );
    }

    #[test]
    fn right_clicking_outside_the_selection_replaces_it() {
        let rows = ids(6);
        let mut selection = Selection::default();
        selection.select(1, 1);
        selection.extend_to_ids(3, &rows);

        right_click(&mut selection, 5, &rows);

        assert_eq!(
            selection.ordered_ids(&rows),
            vec![5],
            "the menu would have acted on rows the user did not point at"
        );
    }

    #[test]
    fn clearing_forgets_the_pivot_too() {
        let rows = ids(5);
        let mut selection = Selection::default();

        selection.select(1, 1);
        selection.clear();
        selection.extend_to_ids(3, &rows);

        assert_eq!(selection.ordered_ids(&rows), vec![3]);
    }
}
