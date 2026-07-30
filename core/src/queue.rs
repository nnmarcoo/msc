//! Playback order: what has played, what is playing, and what comes next.
//!
//! Holds track ids rather than tracks, so reordering never touches metadata and
//! the queue stays valid across a rescan.
//!
//! Two ways to move forward, and they are not interchangeable. [`Queue::advance`]
//! honours the loop mode and is what finishing a track uses. [`Queue::skip`]
//! ignores it, and is what an unplayable track uses: under `Single` an advance
//! would retry the same broken file forever, and under `Queue` it would cycle
//! the whole list looking for something playable.
//!
//! [`Queue::jump_to`] abandons the interrupted track rather than recording it as
//! played, so stepping back returns to whatever genuinely finished before it.
//! The queue is a plan; overriding what plays now does not consume it.
//!
//! [`Queue::reorder`] takes both indices into `upcoming` alone, like
//! [`Queue::remove`], and `to` is where the track ends up *after* it has been
//! lifted out. Removing first means the destination is an index into a list one
//! shorter, so dragging a track down by one lands it back where it started
//! unless the caller accounts for that; the clamp against the shortened length
//! is what keeps a drag to the very end in range rather than doing nothing.

use rand::seq::SliceRandom;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopMode {
    #[default]
    None,
    Queue,
    Single,
}

#[derive(Default)]
pub struct Queue {
    history: VecDeque<i64>,
    current: Option<i64>,
    upcoming: VecDeque<i64>,
    loop_mode: LoopMode,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> Option<i64> {
        self.current
    }

    pub fn upcoming(&self) -> &VecDeque<i64> {
        &self.upcoming
    }

    pub fn history(&self) -> &VecDeque<i64> {
        &self.history
    }

    pub fn is_empty(&self) -> bool {
        self.current.is_none() && self.upcoming.is_empty()
    }

    pub fn len(&self) -> usize {
        self.upcoming.len() + usize::from(self.current.is_some())
    }

    pub fn advance(&mut self) -> Option<i64> {
        match self.loop_mode {
            LoopMode::Single => self.current,
            LoopMode::None => self.step(),
            LoopMode::Queue => self.step().or_else(|| {
                self.restart_cycle();
                self.step()
            }),
        }
    }

    fn restart_cycle(&mut self) {
        self.upcoming.extend(self.history.drain(..));
        if let Some(just_finished) = self.current.take() {
            self.upcoming.push_front(just_finished);
        }
    }

    pub fn go_back(&mut self) -> Option<i64> {
        if let Some(previous) = self.history.pop_back() {
            if let Some(current) = self.current.take() {
                self.upcoming.push_front(current);
            }
            self.current = Some(previous);
        }
        self.current
    }

    pub fn skip(&mut self) -> Option<i64> {
        self.step()
    }

    fn step(&mut self) -> Option<i64> {
        let next = self.upcoming.pop_front()?;
        if let Some(current) = self.current.take() {
            self.history.push_back(current);
        }
        self.current = Some(next);
        self.current
    }

    pub fn jump_to(&mut self, track_id: i64) {
        self.current = Some(track_id);
    }

    pub fn push(&mut self, track_id: i64) {
        match self.current {
            None => self.current = Some(track_id),
            Some(_) => self.upcoming.push_back(track_id),
        }
    }

    pub fn push_next(&mut self, track_id: i64) {
        match self.current {
            None => self.current = Some(track_id),
            Some(_) => self.upcoming.push_front(track_id),
        }
    }

    pub fn extend(&mut self, track_ids: impl IntoIterator<Item = i64>) {
        let mut ids = track_ids.into_iter();
        if self.current.is_none() {
            self.current = ids.next();
        }
        self.upcoming.extend(ids);
    }

    pub fn extend_next(&mut self, track_ids: impl IntoIterator<Item = i64>) {
        let mut incoming: VecDeque<i64> = track_ids.into_iter().collect();
        if self.current.is_none() {
            self.current = incoming.pop_front();
        }
        incoming.extend(self.upcoming.drain(..));
        self.upcoming = incoming;
    }

    pub fn remove(&mut self, index: usize) -> Option<i64> {
        self.upcoming.remove(index)
    }

    pub fn move_to_front(&mut self, index: usize) {
        if let Some(track_id) = self.upcoming.remove(index) {
            self.upcoming.push_front(track_id);
        }
    }

    pub fn reorder(&mut self, from: usize, to: usize) {
        if from >= self.upcoming.len() {
            return;
        }
        let to = to.min(self.upcoming.len() - 1);
        if from == to {
            return;
        }
        if let Some(track_id) = self.upcoming.remove(from) {
            self.upcoming.insert(to, track_id);
        }
    }

    pub fn shuffle(&mut self) {
        let mut upcoming: Vec<i64> = self.upcoming.drain(..).collect();
        upcoming.shuffle(&mut rand::rng());
        self.upcoming = upcoming.into();
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.upcoming.clear();
        self.current = None;
    }

    pub fn remove_track(&mut self, track_id: i64) {
        self.history.retain(|&id| id != track_id);
        self.upcoming.retain(|&id| id != track_id);
        if self.current == Some(track_id) {
            self.current = None;
        }
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }

    pub fn set_loop_mode(&mut self, mode: LoopMode) {
        self.loop_mode = mode;
    }

    pub fn cycle_loop_mode(&mut self) -> LoopMode {
        self.loop_mode = match self.loop_mode {
            LoopMode::None => LoopMode::Queue,
            LoopMode::Queue => LoopMode::Single,
            LoopMode::Single => LoopMode::None,
        };
        self.loop_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queued(upcoming: &[i64]) -> Queue {
        let mut queue = Queue::new();
        queue.current = Some(1);
        queue.upcoming = upcoming.iter().copied().collect();
        queue
    }

    fn order(queue: &Queue) -> Vec<i64> {
        queue.upcoming.iter().copied().collect()
    }

    #[test]
    fn dragging_a_track_up_puts_it_at_the_destination() {
        let mut queue = queued(&[10, 11, 12, 13]);
        queue.reorder(2, 0);

        assert_eq!(order(&queue), vec![12, 10, 11, 13]);
    }

    #[test]
    fn dragging_a_track_down_puts_it_at_the_destination() {
        let mut queue = queued(&[10, 11, 12, 13]);
        queue.reorder(0, 2);

        assert_eq!(order(&queue), vec![11, 12, 10, 13]);
    }

    #[test]
    fn dragging_to_the_end_lands_last() {
        let mut queue = queued(&[10, 11, 12]);
        queue.reorder(0, 2);

        assert_eq!(order(&queue), vec![11, 12, 10]);
    }

    #[test]
    fn dragging_a_track_onto_itself_changes_nothing() {
        let mut queue = queued(&[10, 11, 12]);
        queue.reorder(1, 1);

        assert_eq!(order(&queue), vec![10, 11, 12]);
    }

    #[test]
    fn a_destination_past_the_end_clamps_rather_than_dropping_the_track() {
        let mut queue = queued(&[10, 11, 12]);
        queue.reorder(0, 99);

        assert_eq!(
            order(&queue),
            vec![11, 12, 10],
            "the track should land last, not vanish"
        );
    }

    #[test]
    fn reordering_from_past_the_end_is_ignored() {
        let mut queue = queued(&[10, 11]);
        queue.reorder(7, 0);

        assert_eq!(order(&queue), vec![10, 11]);
    }

    #[test]
    fn reordering_an_empty_queue_does_nothing() {
        let mut queue = queued(&[]);
        queue.reorder(0, 0);

        assert!(order(&queue).is_empty());
    }

    #[test]
    fn reordering_never_loses_or_duplicates_a_track() {
        for from in 0..4 {
            for to in 0..4 {
                let mut queue = queued(&[10, 11, 12, 13]);
                queue.reorder(from, to);

                let mut sorted = order(&queue);
                sorted.sort_unstable();
                assert_eq!(
                    sorted,
                    vec![10, 11, 12, 13],
                    "reorder({from}, {to}) changed the set of queued tracks"
                );
            }
        }
    }

    #[test]
    fn reordering_leaves_the_playing_track_alone() {
        let mut queue = queued(&[10, 11, 12]);
        queue.reorder(0, 2);

        assert_eq!(
            queue.current(),
            Some(1),
            "a reorder touched what was playing"
        );
    }

    #[test]
    fn reordering_does_not_disturb_history() {
        let mut queue = queued(&[10, 11]);
        queue.history = [7, 8].into_iter().collect();
        queue.reorder(0, 1);

        assert_eq!(
            queue.history().iter().copied().collect::<Vec<_>>(),
            vec![7, 8]
        );
    }
}
