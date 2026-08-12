//! A bounded, expiring cache for search results.
//!
//! Debounced typing still issues a request per pause, and a user comparing two
//! artists searches the same pair repeatedly, so the same few queries dominate
//! the traffic. A short lifetime keeps results fresh enough for a catalogue
//! that barely changes.
//!
//! It is a `Mutex<HashMap>` rather than anything lock-free because it is read on
//! a keystroke or a click, never per frame, and it must be shared across the
//! tasks a search spawns.
//!
//! Overflow evicts the least recently *used* entry, not the oldest and not
//! everything. The distinction matters more than the memory it saves: a cache
//! that clears itself when full drops the entry for the query being typed right
//! then, since a steady session keeps every entry live and nothing is expired
//! to reclaim. Refetching the one query the user is actively working on is the
//! worst possible choice, and it is the one an "expire, else clear" policy
//! makes every time it fills.
//!
//! Least-recently-used is tracked by a counter bumped on read, rather than by
//! an intrusive list. Eviction is a scan of at most [`CAPACITY`] entries and
//! happens only when full, whereas a list would pay pointer maintenance on
//! every hit; at this size and call rate the scan is cheaper and far less code.
//!
//! A poisoned lock is recovered from rather than propagated. The data is a
//! cache of public search results — nothing here has an invariant a panic could
//! have broken, and faulting every later search because one unrelated task
//! panicked would turn a recoverable failure into a dead pane.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::PoisonError;
use std::time::{Duration, Instant};

const LIFETIME: Duration = Duration::from_secs(300);
const CAPACITY: usize = 256;

struct Entry<T> {
    stored: Instant,
    used: u64,
    value: T,
}

pub struct Cache<T> {
    entries: Mutex<Held<T>>,
}

struct Held<T> {
    map: HashMap<String, Entry<T>>,
    clock: u64,
}

impl<T: Clone> Default for Cache<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Cache<T> {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Held {
                map: HashMap::new(),
                clock: 0,
            }),
        }
    }

    fn held(&self) -> std::sync::MutexGuard<'_, Held<T>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn get(&self, key: &str) -> Option<T> {
        let mut held = self.held();

        held.clock += 1;
        let now = held.clock;

        let entry = held.map.get_mut(key)?;
        if entry.stored.elapsed() >= LIFETIME {
            held.map.remove(key);
            return None;
        }

        entry.used = now;
        Some(entry.value.clone())
    }

    pub fn put(&self, key: String, value: T) {
        let mut held = self.held();

        held.clock += 1;
        let now = held.clock;

        if held.map.len() >= CAPACITY && !held.map.contains_key(&key) {
            held.map.retain(|_, e| e.stored.elapsed() < LIFETIME);

            if held.map.len() >= CAPACITY
                && let Some(stalest) = held
                    .map
                    .iter()
                    .min_by_key(|(_, e)| e.used)
                    .map(|(k, _)| k.clone())
            {
                held.map.remove(&stalest);
            }
        }

        held.map.insert(
            key,
            Entry {
                stored: Instant::now(),
                used: now,
                value,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fill(cache: &Cache<usize>, count: usize) {
        for i in 0..count {
            cache.put(format!("k{i}"), i);
        }
    }

    #[test]
    fn what_went_in_comes_back() {
        let cache = Cache::new();
        cache.put("k".to_owned(), vec![1, 2, 3]);

        assert_eq!(cache.get("k"), Some(vec![1, 2, 3]));
    }

    #[test]
    fn a_key_never_stored_is_absent() {
        let cache: Cache<u8> = Cache::new();
        assert_eq!(cache.get("nothing"), None);
    }

    #[test]
    fn storing_a_key_again_replaces_it() {
        let cache = Cache::new();
        cache.put("k".to_owned(), 1);
        cache.put("k".to_owned(), 2);

        assert_eq!(cache.get("k"), Some(2));
    }

    #[test]
    fn filling_it_past_capacity_bounds_it() {
        let cache = Cache::new();
        fill(&cache, CAPACITY + 10);

        let held = cache.held().map.len();
        assert!(held <= CAPACITY, "held {held} entries, cap is {CAPACITY}");
    }

    #[test]
    fn overflow_keeps_what_is_being_used() {
        let cache = Cache::new();
        fill(&cache, CAPACITY);

        assert_eq!(cache.get("k0"), Some(0));

        cache.put("newcomer".to_owned(), 999);

        assert_eq!(cache.get("k0"), Some(0), "the hot entry survived");
        assert_eq!(cache.get("newcomer"), Some(999));
    }

    #[test]
    fn overflow_evicts_one_entry_rather_than_everything() {
        let cache = Cache::new();
        fill(&cache, CAPACITY);
        cache.put("newcomer".to_owned(), 999);

        let held = cache.held().map.len();
        assert_eq!(held, CAPACITY, "one out, one in");
    }

    #[test]
    fn the_least_recently_read_entry_is_the_one_dropped() {
        let cache = Cache::new();
        fill(&cache, CAPACITY);

        for i in 0..CAPACITY {
            if i != 7 {
                let _ = cache.get(&format!("k{i}"));
            }
        }

        cache.put("newcomer".to_owned(), 999);

        assert_eq!(cache.get("k7"), None, "the untouched entry went");
        assert_eq!(cache.get("k0"), Some(0));
    }

    #[test]
    fn replacing_a_key_at_capacity_evicts_nothing() {
        let cache = Cache::new();
        fill(&cache, CAPACITY);

        cache.put("k5".to_owned(), 555);

        assert_eq!(cache.held().map.len(), CAPACITY);
        assert_eq!(cache.get("k5"), Some(555));
        assert_eq!(cache.get("k0"), Some(0));
    }

    #[test]
    fn a_poisoned_lock_still_serves() {
        use std::sync::Arc;

        let cache = Arc::new(Cache::new());
        cache.put("k".to_owned(), 7);

        let poisoner = Arc::clone(&cache);
        let _ = std::thread::spawn(move || {
            let _guard = poisoner.entries.lock().expect("held");
            panic!("poison the lock");
        })
        .join();

        assert_eq!(cache.get("k"), Some(7));
        cache.put("j".to_owned(), 8);
        assert_eq!(cache.get("j"), Some(8));
    }
}
