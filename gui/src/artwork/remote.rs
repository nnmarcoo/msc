//! Cover art for recordings that are not on disk yet.
//!
//! [`super::Cache`] is keyed on a track id and a file path, because everything
//! it holds was read out of a file the library owns. A search result has
//! neither: it is a URL, and it may never become a file at all. Rather than
//! widen that cache's key to something most of it cannot use, remote art is its
//! own small store keyed by URL.
//!
//! What it does share is the shape of the problem. `view` runs per frame and
//! cannot fetch, so asking for a cover records the want and answers with
//! whatever is ready; the app drains [`Remote::take`] after `update` and turns
//! each want into one task. A URL already wanted is not wanted twice, and one
//! that has answered — with an image or with nothing — is never asked again, so
//! a cover the host will not serve costs one request rather than one per frame.
//!
//! Sizes are not bucketed the way local art is. Google's image host takes the
//! dimensions in the URL, so the caller asks for the size it wants at request
//! time and every distinct size is a distinct URL; there is no master to rescale
//! from and nothing to quantise.

use std::cell::RefCell;
use std::collections::HashSet;
use std::num::NonZeroUsize;

use iced::widget::image::Handle;
use lru::LruCache;

const CAPACITY: usize = 128;

#[derive(Debug, Default)]
pub struct Remote {
    ready: RefCell<Option<LruCache<String, Option<Handle>>>>,
    wanted: RefCell<Vec<String>>,
    asked: RefCell<HashSet<String>>,
}

impl Remote {
    pub fn new() -> Self {
        Self {
            ready: RefCell::new(Some(LruCache::new(
                NonZeroUsize::new(CAPACITY).expect("non-zero"),
            ))),
            wanted: RefCell::new(Vec::new()),
            asked: RefCell::new(HashSet::new()),
        }
    }

    pub fn request(&self, url: &str) -> Option<Handle> {
        let mut ready = self.ready.borrow_mut();
        let cache = ready.as_mut()?;

        if let Some(entry) = cache.get(url) {
            return entry.clone();
        }

        if !self.asked.borrow_mut().insert(url.to_owned()) {
            return None;
        }

        self.wanted.borrow_mut().push(url.to_owned());
        None
    }

    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut self.wanted.borrow_mut())
    }

    pub fn insert(&self, url: String, handle: Option<Handle>) {
        if let Some(cache) = self.ready.borrow_mut().as_mut() {
            cache.put(url, handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle() -> Handle {
        Handle::from_rgba(1, 1, vec![0, 0, 0, 255])
    }

    #[test]
    fn a_url_never_seen_is_wanted_once() {
        let remote = Remote::new();

        assert!(remote.request("https://host/a").is_none());
        assert_eq!(remote.take(), vec!["https://host/a".to_owned()]);
    }

    #[test]
    fn asking_twice_before_it_answers_wants_it_once() {
        let remote = Remote::new();

        remote.request("https://host/a");
        remote.request("https://host/a");

        assert_eq!(remote.take().len(), 1);
    }

    #[test]
    fn what_arrived_is_handed_back() {
        let remote = Remote::new();

        remote.request("https://host/a");
        remote.take();
        remote.insert("https://host/a".to_owned(), Some(handle()));

        assert!(remote.request("https://host/a").is_some());
        assert!(remote.take().is_empty());
    }

    #[test]
    fn a_cover_the_host_would_not_serve_is_not_asked_for_again() {
        let remote = Remote::new();

        remote.request("https://host/gone");
        remote.take();
        remote.insert("https://host/gone".to_owned(), None);

        assert!(remote.request("https://host/gone").is_none());
        assert!(
            remote.take().is_empty(),
            "a failed cover must not be requested once per frame"
        );
    }

    #[test]
    fn taking_the_wants_clears_them() {
        let remote = Remote::new();

        remote.request("https://host/a");
        assert_eq!(remote.take().len(), 1);
        assert!(remote.take().is_empty());
    }
}
