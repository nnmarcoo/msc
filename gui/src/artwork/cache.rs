//! Cover art, decoded off the frame thread and kept at the sizes drawn.
//!
//! `Handle::from_rgba` stamps each handle with a fresh id, and the wgpu renderer
//! keys its texture atlas on that id and drops whatever it did not see last
//! frame. Building a handle inside `view` is therefore a texture upload every
//! frame, at the 16ms tick, so handles are built once and stored; cloning one is
//! cheap, since the pixels behind it are reference counted.
//!
//! Pixels are keyed on a hash of the encoded picture bytes, so tracks from one
//! album collapse to a single entry without anything having to decide they are
//! the same album, and the odd track carrying different art stays separate for
//! the same reason. `sources` maps a track to that key and is the only per-track
//! state; `None` records a track with no art so it is read once rather than once
//! per frame.
//!
//! `masters` and `scaled` split full-resolution images from the handles panes
//! draw, because they are wanted for different lengths of time: a master only to
//! cut a *new* size, a handle on every frame that draws it. So a master is what
//! should go when memory is short, and one cover is decoded once however many
//! tracks carry it. Masters are bounded by bytes rather than count, their sizes
//! differing by orders of magnitude, and shared by [`Arc`] because cloning a
//! 4000x4000 image costs a whole frame.
//!
//! Requests are rounded up to [`LADDER`], since keying on exact pixel widths
//! would resample and re-upload on every pixel of a divider drag. The ladder is
//! capped *down* to the source's own longest edge, so nothing is ever enlarged
//! and one image is never filed under two buckets; [`Cache::request`] may
//! therefore answer smaller than asked, and panes must draw what they are given.
//!
//! Decoding costs tens of milliseconds, so `view` only records what is missing
//! and [`Cache::take`] drains that after every message, whatever the message was.
//! Draining on the playback tick alone left a grid of covers queued forever with
//! nothing playing, since that tick does not run then. `pending` keeps a miss
//! redrawn sixty times a second down to one job. `request` takes `&self` because `view`
//! has only `&self`, which keeps [`crate::pane::view::Shared`] `Copy`; it must
//! touch a hit rather than peek, so anything on screen is too recently used to be
//! evicted beneath the pane drawing it.

//!
//! Between a track changing and its art arriving this answers `None`. Bridging
//! that gap belongs to the pane, being a fact about what one pane drew last
//! rather than about any image; see [`crate::pane::artwork`].

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::widget::image::Handle;
use lru::LruCache;

const LADDER: [u32; 6] = [64, 128, 256, 512, 1024, 2048];
const MASTER_BUDGET: usize = 96 * 1024 * 1024;
const SCALED_CAPACITY: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtKey(u64);

impl ArtKey {
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        Self(hasher.finish())
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub track: i64,
    pub path: PathBuf,
    pub bucket: u32,
}

pub type Source = (ArtKey, Arc<image::RgbaImage>);

#[derive(Debug, Clone)]
pub struct Art {
    pub track: i64,
    pub key: Option<ArtKey>,
    pub bucket: u32,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub source_edge: u32,
}

struct Master {
    image: Arc<image::RgbaImage>,
    bytes: usize,
}

pub struct Cache {
    sources: HashMap<i64, Option<ArtKey>>,
    extents: HashMap<ArtKey, u32>,
    masters: RefCell<LruCache<ArtKey, Master>>,
    master_bytes: RefCell<usize>,
    scaled: RefCell<LruCache<(ArtKey, u32), Handle>>,
    wanted: RefCell<Vec<Job>>,
    pending: RefCell<Vec<(i64, u32)>>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            extents: HashMap::new(),
            masters: RefCell::new(LruCache::unbounded()),
            master_bytes: RefCell::new(0),
            scaled: RefCell::new(LruCache::new(
                NonZeroUsize::new(SCALED_CAPACITY).expect("non-zero"),
            )),
            wanted: RefCell::new(Vec::new()),
            pending: RefCell::new(Vec::new()),
        }
    }

    pub fn request(&self, track: i64, path: &Path, edge: f32) -> Option<Handle> {
        let source = self.sources.get(&track).copied();
        if source == Some(None) {
            return None;
        }

        let key = source.flatten();
        let bucket = key.map_or_else(|| quantise(edge), |key| self.bucket_for(key, edge));

        if let Some(key) = key
            && let Some(handle) = self.scaled.borrow_mut().get(&(key, bucket))
        {
            return Some(handle.clone());
        }

        self.want(track, path, bucket);

        key.and_then(|key| self.nearest_ready(key, bucket))
    }

    pub fn resolved_empty(&self, track: i64) -> bool {
        self.sources.get(&track) == Some(&None)
    }

    pub fn is_idle(&self) -> bool {
        self.wanted.borrow().is_empty()
    }

    pub fn take(&mut self) -> Vec<(Job, Option<Source>)> {
        let jobs = std::mem::take(&mut *self.wanted.borrow_mut());
        self.pending
            .borrow_mut()
            .extend(jobs.iter().map(|job| (job.track, job.bucket)));

        jobs.into_iter()
            .map(|job| {
                let master = self
                    .source_of(job.track)
                    .and_then(|key| Some((key, self.master(key)?)));
                (job, master)
            })
            .collect()
    }

    pub fn insert(&mut self, art: Art) {
        self.sources.insert(art.track, art.key);

        let Some(key) = art.key else {
            self.pending
                .borrow_mut()
                .retain(|(track, _)| *track != art.track);
            return;
        };

        let sources = &self.sources;
        self.pending.borrow_mut().retain(|(track, bucket)| {
            let same_image = sources.get(track).copied().flatten() == Some(key);
            !(same_image && *bucket == art.bucket)
        });

        self.extents.insert(key, art.source_edge);
        let handle = Handle::from_rgba(art.width, art.height, art.pixels);
        self.scaled.borrow_mut().put((key, art.bucket), handle);
    }

    pub fn keep_master(&mut self, key: ArtKey, image: Arc<image::RgbaImage>) {
        let bytes = image.width() as usize * image.height() as usize * 4;
        if bytes > MASTER_BUDGET {
            return;
        }

        let mut masters = self.masters.borrow_mut();
        let mut total = self.master_bytes.borrow_mut();

        while *total + bytes > MASTER_BUDGET {
            match masters.pop_lru() {
                Some((_, evicted)) => *total = total.saturating_sub(evicted.bytes),
                None => break,
            }
        }

        if let Some(replaced) = masters.put(key, Master { image, bytes }) {
            *total = total.saturating_sub(replaced.bytes);
        }
        *total += bytes;
    }

    pub fn forget(&mut self, live: &[i64]) {
        let live: HashSet<i64> = live.iter().copied().collect();
        self.sources.retain(|id, _| live.contains(id));

        let reachable: HashSet<ArtKey> = self.sources.values().flatten().copied().collect();
        self.extents.retain(|key, _| reachable.contains(key));

        let mut masters = self.masters.borrow_mut();
        let mut total = self.master_bytes.borrow_mut();
        let stale: Vec<ArtKey> = masters
            .iter()
            .map(|(key, _)| *key)
            .filter(|key| !reachable.contains(key))
            .collect();
        for key in stale {
            if let Some(dropped) = masters.pop(&key) {
                *total = total.saturating_sub(dropped.bytes);
            }
        }

        let mut scaled = self.scaled.borrow_mut();
        let stale: Vec<(ArtKey, u32)> = scaled
            .iter()
            .map(|(entry, _)| *entry)
            .filter(|(key, _)| !reachable.contains(key))
            .collect();
        for entry in stale {
            scaled.pop(&entry);
        }
    }

    fn source_of(&self, track: i64) -> Option<ArtKey> {
        self.sources.get(&track).copied().flatten()
    }

    fn want(&self, track: i64, path: &Path, bucket: u32) {
        let in_flight = self.pending.borrow().contains(&(track, bucket));
        let queued = self
            .wanted
            .borrow()
            .iter()
            .any(|job| job.track == track && job.bucket == bucket);
        if in_flight || queued {
            return;
        }

        self.wanted.borrow_mut().push(Job {
            track,
            path: path.to_path_buf(),
            bucket,
        });
    }

    fn master(&self, key: ArtKey) -> Option<Arc<image::RgbaImage>> {
        self.masters
            .borrow_mut()
            .get(&key)
            .map(|master| Arc::clone(&master.image))
    }

    fn bucket_for(&self, key: ArtKey, edge: f32) -> u32 {
        let wanted = quantise(edge);
        match self.extents.get(&key) {
            Some(&extent) => wanted.min(cap(extent)),
            None => wanted,
        }
    }

    fn nearest_ready(&self, key: ArtKey, wanted: u32) -> Option<Handle> {
        self.scaled
            .borrow()
            .iter()
            .filter(|((candidate, _), _)| *candidate == key)
            .min_by_key(|((_, bucket), _)| rank(*bucket, wanted))
            .map(|(_, handle)| handle.clone())
    }
}

fn quantise(edge: f32) -> u32 {
    let edge = edge.max(1.0) as u32;
    LADDER
        .iter()
        .copied()
        .find(|&step| step >= edge)
        .unwrap_or(LADDER[LADDER.len() - 1])
}

fn cap(extent: u32) -> u32 {
    LADDER
        .iter()
        .copied()
        .rev()
        .find(|&step| step <= extent)
        .unwrap_or(LADDER[0])
}

fn rank(candidate: u32, wanted: u32) -> (u8, u32) {
    if candidate >= wanted {
        (0, candidate - wanted)
    } else {
        (1, wanted - candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "a.mp3";
    const B: &str = "b.mp3";

    fn path(name: &str) -> &Path {
        Path::new(name)
    }

    fn art(track: i64, key: ArtKey, bucket: u32) -> Art {
        Art {
            track,
            key: Some(key),
            bucket,
            width: bucket,
            height: bucket,
            pixels: vec![0; bucket as usize * bucket as usize * 4],
            source_edge: 3000,
        }
    }

    fn nothing(track: i64, bucket: u32) -> Art {
        Art {
            track,
            key: None,
            bucket,
            width: 0,
            height: 0,
            pixels: Vec::new(),
            source_edge: 0,
        }
    }

    #[test]
    fn identical_bytes_share_a_key() {
        assert_eq!(ArtKey::of(&[1, 2, 3]), ArtKey::of(&[1, 2, 3]));
        assert_ne!(ArtKey::of(&[1, 2, 3]), ArtKey::of(&[1, 2, 4]));
    }

    #[test]
    fn a_request_rounds_up_to_the_ladder() {
        assert_eq!(quantise(1.0), 64);
        assert_eq!(quantise(64.0), 64);
        assert_eq!(quantise(65.0), 128);
        assert_eq!(quantise(99_999.0), 2048);
    }

    #[test]
    fn nearby_pane_sizes_share_one_bucket() {
        let buckets: Vec<u32> = [257.0, 300.0, 400.0, 511.0, 512.0]
            .iter()
            .map(|&edge| quantise(edge))
            .collect();

        assert!(
            buckets.iter().all(|&bucket| bucket == 512),
            "a divider drag crossed buckets at {buckets:?}, so it would resample per pixel"
        );
    }

    #[test]
    fn no_source_is_ever_filed_above_its_own_size() {
        let mut cache = Cache::new();

        for extent in [50u32, 64, 300, 599, 1280, 2000, 4000] {
            let key = ArtKey::of(&extent.to_le_bytes());
            cache.extents.insert(key, extent);

            let bucket = cache.bucket_for(key, 4096.0);
            assert!(
                bucket <= extent.max(LADDER[0]),
                "a {extent}px cover was filed under {bucket}, so the same pixels \
                 could be held twice and a smaller request would miss them"
            );
        }
    }

    #[test]
    fn a_request_below_the_cap_is_untouched_by_it() {
        let mut cache = Cache::new();
        let key = ArtKey::of(&[7]);
        cache.extents.insert(key, 1280);

        assert_eq!(cache.bucket_for(key, 200.0), 256);
    }

    #[test]
    fn a_miss_is_only_queued_once_while_it_is_in_flight() {
        let mut cache = Cache::new();

        for _ in 0..60 {
            let _ = cache.request(1, path(A), 200.0);
        }
        assert_eq!(
            cache.take().len(),
            1,
            "sixty frames queued more than one job"
        );

        for _ in 0..60 {
            let _ = cache.request(1, path(A), 200.0);
        }
        assert!(cache.take().is_empty(), "a job in flight was queued again");
    }

    #[test]
    fn a_filed_result_answers_the_next_request() {
        let mut cache = Cache::new();
        let key = ArtKey::of(&[1]);

        let _ = cache.request(1, path(A), 200.0);
        let _ = cache.take();
        cache.insert(art(1, key, 256));

        assert!(cache.request(1, path(A), 200.0).is_some());
        assert!(cache.take().is_empty(), "a cached size was queued again");
    }

    #[test]
    fn a_further_size_is_cut_from_the_master_rather_than_re_read() {
        let mut cache = Cache::new();
        let key = ArtKey::of(&[1]);

        let _ = cache.request(1, path(A), 200.0);
        let _ = cache.take();
        cache.insert(art(1, key, 256));
        cache.keep_master(key, Arc::new(image::RgbaImage::new(1000, 1000)));

        let _ = cache.request(1, path(A), 900.0);
        let handed = cache.take();

        assert_eq!(handed.len(), 1);
        assert!(
            handed[0].1.is_some(),
            "a size for an image already decoded was not cut from its master"
        );
    }

    #[test]
    fn a_handed_master_is_shared_rather_than_copied() {
        let mut cache = Cache::new();
        let key = ArtKey::of(&[1]);
        cache.insert(art(1, key, 256));
        cache.keep_master(key, Arc::new(image::RgbaImage::new(64, 64)));

        let held = cache.master(key).expect("just kept");
        let again = cache.master(key).expect("still kept");

        assert!(
            Arc::ptr_eq(&held, &again),
            "handing out a master copied it, which for a large cover costs a frame"
        );
    }

    #[test]
    fn a_track_sharing_a_cover_draws_it_without_any_work() {
        let mut cache = Cache::new();
        let key = ArtKey::of(&[1]);

        let _ = cache.request(1, path(A), 200.0);
        let _ = cache.take();
        cache.insert(art(1, key, 256));
        cache.insert(art(2, key, 256));

        assert!(
            cache.request(2, path(B), 200.0).is_some(),
            "the second track did not reuse a cover already decoded"
        );
        assert!(
            cache.take().is_empty(),
            "it queued work for art already held"
        );
    }

    #[test]
    fn filing_one_cover_answers_every_track_waiting_on_it() {
        let mut cache = Cache::new();
        let key = ArtKey::of(&[1]);

        let _ = cache.request(1, path(A), 200.0);
        let _ = cache.request(2, path(B), 200.0);
        assert_eq!(cache.take().len(), 2);

        cache.insert(art(1, key, 256));
        cache.insert(art(2, key, 256));

        assert!(cache.request(2, path(B), 200.0).is_some());
        assert!(cache.take().is_empty());
    }

    #[test]
    fn a_queued_cover_reports_itself_as_work_to_do() {
        let cache = Cache::new();
        assert!(cache.is_idle(), "an untouched cache claimed work");

        let _ = cache.request(1, path(A), 200.0);

        assert!(
            !cache.is_idle(),
            "a queued cover left the cache looking idle, so the drain would be \
             skipped and the request never spawned"
        );
    }

    #[test]
    fn draining_returns_the_cache_to_idle() {
        let mut cache = Cache::new();
        let _ = cache.request(1, path(A), 200.0);
        let _ = cache.take();

        assert!(
            cache.is_idle(),
            "a drained cache still claimed work, so every message would rebuild \
             a task batch for nothing"
        );
    }

    #[test]
    fn a_failed_decode_does_not_wedge_the_track() {
        let mut cache = Cache::new();

        let _ = cache.request(1, path(A), 200.0);
        assert_eq!(cache.take().len(), 1);
        cache.insert(nothing(1, 256));

        assert!(
            cache.pending.borrow().is_empty(),
            "a track left in flight would never be asked for again"
        );
    }

    #[test]
    fn a_track_with_no_art_is_read_once() {
        let mut cache = Cache::new();

        let _ = cache.request(1, path(A), 200.0);
        let _ = cache.take();
        cache.insert(nothing(1, 256));

        assert!(cache.request(1, path(A), 200.0).is_none());
        assert!(
            cache.take().is_empty(),
            "a track known to carry no art was read again"
        );
        assert!(cache.resolved_empty(1));
    }

    #[test]
    fn an_unread_track_is_not_mistaken_for_one_without_art() {
        let cache = Cache::new();
        assert!(!cache.resolved_empty(1));
    }

    #[test]
    fn one_image_at_two_sizes_keeps_both() {
        let mut cache = Cache::new();
        let key = ArtKey::of(&[1]);

        cache.insert(art(1, key, 128));
        cache.insert(art(1, key, 512));

        let _ = cache.request(1, path(A), 100.0);
        let _ = cache.request(1, path(A), 500.0);

        assert!(cache.take().is_empty(), "two held sizes both queued work");
        assert_eq!(cache.scaled.borrow().len(), 2);
    }

    #[test]
    fn a_pending_size_still_draws_a_size_already_held() {
        let mut cache = Cache::new();
        let key = ArtKey::of(&[1]);
        cache.insert(art(1, key, 128));

        assert!(
            cache.request(1, path(A), 500.0).is_some(),
            "a pane growing past its cached size fell back to nothing"
        );
    }

    #[test]
    fn a_larger_held_size_is_preferred_to_a_smaller_one() {
        assert!(rank(512, 256) < rank(128, 256));
        assert!(rank(256, 256) < rank(512, 256));
        assert!(rank(1024, 256) > rank(512, 256));
    }

    #[test]
    fn the_master_budget_is_not_exceeded() {
        let mut cache = Cache::new();

        for n in 0..8u8 {
            cache.keep_master(
                ArtKey::of(&[n]),
                Arc::new(image::RgbaImage::new(2048, 2048)),
            );
        }

        assert!(
            *cache.master_bytes.borrow() <= MASTER_BUDGET,
            "masters grew past the budget"
        );
    }

    #[test]
    fn an_oversized_master_is_refused_rather_than_emptying_the_cache() {
        let mut cache = Cache::new();
        cache.keep_master(
            ArtKey::of(&[1]),
            Arc::new(image::RgbaImage::new(8192, 8192)),
        );

        assert!(*cache.master_bytes.borrow() <= MASTER_BUDGET);
    }

    #[test]
    fn evicting_a_master_keeps_the_handles_cut_from_it() {
        let mut cache = Cache::new();
        let key = ArtKey::of(&[1]);

        cache.insert(art(1, key, 256));
        cache.keep_master(key, Arc::new(image::RgbaImage::new(64, 64)));

        for n in 1..8u8 {
            cache.keep_master(
                ArtKey::of(&[n + 10]),
                Arc::new(image::RgbaImage::new(2048, 2048)),
            );
        }

        assert!(
            cache.request(1, path(A), 200.0).is_some(),
            "evicting the master took the drawable handle with it"
        );
    }

    #[test]
    fn a_rescan_forgets_tracks_that_are_gone() {
        let mut cache = Cache::new();
        cache.insert(art(1, ArtKey::of(&[1]), 256));
        cache.insert(art(2, ArtKey::of(&[2]), 256));

        cache.forget(&[1]);

        assert!(cache.sources.contains_key(&1));
        assert!(!cache.sources.contains_key(&2));
    }

    #[test]
    fn a_rescan_releases_the_pixels_of_tracks_that_are_gone() {
        let mut cache = Cache::new();
        let (kept, gone) = (ArtKey::of(&[1]), ArtKey::of(&[2]));

        cache.insert(art(1, kept, 256));
        cache.insert(art(2, gone, 256));
        cache.keep_master(kept, Arc::new(image::RgbaImage::new(64, 64)));
        cache.keep_master(gone, Arc::new(image::RgbaImage::new(2048, 2048)));

        let before = *cache.master_bytes.borrow();
        cache.forget(&[1]);

        assert!(
            *cache.master_bytes.borrow() < before,
            "a rescan left the masters of departed tracks resident"
        );
        assert!(cache.master(gone).is_none());
        assert!(!cache.extents.contains_key(&gone));
        assert!(
            cache.scaled.borrow().peek(&(gone, 256)).is_none(),
            "a rescan left drawable copies of art no track claims"
        );

        assert!(cache.master(kept).is_some(), "it dropped art still in use");
        assert!(cache.scaled.borrow().peek(&(kept, 256)).is_some());
    }

    #[test]
    fn a_shared_cover_survives_one_of_its_tracks_leaving() {
        let mut cache = Cache::new();
        let key = ArtKey::of(&[1]);

        cache.insert(art(1, key, 256));
        cache.insert(art(2, key, 256));
        cache.keep_master(key, Arc::new(image::RgbaImage::new(64, 64)));

        cache.forget(&[1]);

        assert!(
            cache.master(key).is_some() && cache.scaled.borrow().peek(&(key, 256)).is_some(),
            "art still carried by a living track was dropped with its sibling"
        );
    }
}
