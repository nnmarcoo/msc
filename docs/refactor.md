# Core refactor plan

Restructuring `verse-core` so playback, library, and configuration each own exactly
one concern. Written against the `exp` branch.

## Decisions

| Question | Decision |
| --- | --- |
| Library placement | Pulled out of `Player`; siblings under the app |
| Player ↔ Library | Borrow at call site — `&Library` passed to the methods that need it |
| Audio crate | Keep kira; no missing control identified |
| Config | Delete the global; adopt the owned pattern already used by `gui/src/config.rs` |
| Library root | Stored in the DB as a `meta(key, value)` row |
| Analyzer | Split into `Meter` + `Spectrum`, both still on the audio thread |
| Keep a DB? | **Yes** — scan is ~400× costlier than load (measured) |
| DB's role | **Persistence only.** Runtime queries served from an in-memory index |
| DB location | Stays in `core` — the TUI needs the same library the GUI does |
| `albums` table | **Dropped** — fully derivable from track tags |
| `missing` flag | **Kept and surfaced in the UI** — protects playlist membership |
| Timestamps | **Dropped** — six columns written, never read |

## Measured baseline

253 tracks / 25 albums (`D:\audio`), release build:

| Operation | Time |
| --- | --- |
| Full scan (walk + lofty parse + sqlite write) | **375 ms** |
| Load all tracks from DB | **0.94 ms** |
| Load all albums from DB | **0.32 ms** |
| `query_track_from_id` (single) | **12.7 µs** |
| `Track` size | 248 B → 253 tracks ≈ 0.1 MB |

Three conclusions:

1. **Keep the DB.** Scanning is ~400× the cost of loading. Extrapolated to 5k tracks:
   ~7.4 s scan vs ~19 ms load. Rescanning at launch would be a visible stall.
2. **Load everything at startup.** Full load is ~1.3 ms / 0.1 MB (≈25 ms / 2 MB at 5k
   tracks). There is no case for lazy querying at this scale.
3. **The hot path is the real bug.** `query_track_from_id` at 12.7 µs is called from
   `view()` — once per queue row, per frame, at a 30 ms tick. 60 rows = ~762 µs/frame
   (~2.5% of budget) re-fetching in-memory data. A `HashMap` hit is ~20 ns: **~600×
   faster**.

## Target ownership

```
App (gui / tui)
 ├── Config    GUI-only: theme, layouts, volume  (unchanged — already correct)
 ├── Library   In-memory index + Database (write-through persistence)
 └── Player    Backend + Queue only
```

`Player` holds no library state and no config. Methods needing track data take
`&Library` explicitly.

### Library shape

```rust
pub struct Library {
    tracks:    Vec<Track>,
    by_id:     HashMap<i64, usize>,     // replaces query_track_from_id
    by_path:   HashMap<PathBuf, usize>, // replaces query_track_from_path
    albums:    Vec<Album>,              // derived in Rust, not SQL
    playlists: Vec<Playlist>,
    db:        Database,                // load-once + write-through on mutation
    root:      Option<PathBuf>,
}
```

The library *is* the cache. This deletes the GUI's three `RefCell<Option<Vec<..>>>`
fields (`app.rs:36-38`), `invalidate_library_cache`, `invalidate_playlist_cache`, and
their call sites — the `ensure_cached_*` methods become plain accessors.

### Dead API to delete

Never called by any consumer: `query_n_tracks`, `query_track_count`,
`query_tracks_by_artist`, `set_playlist_cover` (exposed through all three layers,
called by nobody). Along with the `forward_queries!` macro, which stops earning its
keep once queries are in-memory accessors.

## Why

Three problems in the current code:

1. **`Player` is a god object.** ~35 methods, of which ~20 are pure pass-throughs to
   `Library` with no playback involvement. The doc comments at `player.rs:28` and
   `library.rs:105` claim the wrappers exist "to keep the queue consistent" — but none
   of them touch the queue today. The invariant is aspirational, not real.

2. **`core::Config` is a global singleton holding one field.** `static OnceLock<RwLock<Config>>`
   for `root: Option<PathBuf>`. It forces an init-ordering hazard in `Player::new`,
   makes tests impossible, hides data flow, and drags `ConfigError` into both
   `PlayerError` and `LibraryError`. `Drop for Player` calling `Config::save_current()`
   is an implicit side effect at teardown.

3. **The analyzer interleaves three jobs** — FFT spectrum, peak metering, RMS metering —
   in one struct, with kira `EffectBuilder` glue mixed into the DSP file.

4. **SQLite is being used as a runtime data structure.** Three layers
   (`Database` → `Library` → `Player`) forward the same call, and the innermost one gets
   invoked from inside `view()`. Meanwhile the GUI maintains its own parallel cache of
   the same data, so the DB is simultaneously too slow (hot path) and redundant
   (cached anyway).

Note: locking is *not* a real problem today. The core is single-threaded by design
(`&mut self` throughout, driven from the GUI update loop). The only cross-thread data is
`VisData`, already handled lock-free by `triple_buffer`; its `Mutex` guards an
uncontended reader handle. This refactor is about ownership and responsibility, not
lock removal.

---

# Step 1 — Config → DB meta

Self-contained inside `core`. Tree stays green; the GUI needs one line changed.

### Core changes

- **Delete `core/src/config.rs`** and the `Config` / `ConfigError` exports from `lib.rs`.
- **New `db/meta.rs`**: `meta(key TEXT PRIMARY KEY, value TEXT)` table, with
  `Database::get_meta(key) -> Option<String>` and `set_meta(key, value)`.
  Add the `CREATE TABLE` to `db/schema.rs` (it is `IF NOT EXISTS`, so existing DBs
  pick it up on next open — no migration needed).
- **`library.rs`**: private helpers `project_dirs()` / `database_path()` replace the
  `Config::` associated functions. `Library` gains:
  - `root(&self) -> Option<PathBuf>` — reads `meta.root`
  - `set_root(&self, PathBuf)` — writes `meta.root`
  - `clear_root(&self)`
  - `scan` / `scan_with_root` become **instance methods** (they currently construct a
    second `Database` — that goes away; they reuse `self.db`).
- **`player.rs`**: drop `Config::init()` from `new()`, drop `impl Drop for Player`,
  drop the `Config` variant from `PlayerError`. `clear_library` is removed here
  (it moves to `Library` in step 3; interim it stays but calls `library.clear()` +
  `library.clear_root()`).
- **`LibraryError`**: `Config(ConfigError)` variant replaced by a `Dirs` variant for
  the "could not resolve project directory" case.

### Call sites

- `gui/src/app.rs:363` — `verse_core::Config::root()` → `self.library.root()` (step 3)
  or `self.player.library().root()` (interim).
- `gui/src/app.rs:359,379` — `Library::scan_with_root(&path)` / `Library::scan` are now
  instance methods, but `spawn_scan` moves them to another thread. **This is the one
  real complication**: see "Scanning" below.

### Scanning

`spawn_scan` (`app.rs:235`) sends the scan to a worker thread. Today that works because
`scan()` is an associated function that opens its *own* `Database`. Making it an instance
method breaks that (`Library` is not `Sync`; `rusqlite::Connection` is not `Sync`).

Options — **recommend (a)**:

- **(a) Keep a standalone scan constructor.** `Library::open_for_scan(root) -> Library`
  opens a second connection on the worker thread and scans. WAL mode is already enabled,
  so a concurrent writer is fine. Preserves today's behavior exactly; the only change is
  that `root` is read from / written to the DB rather than a TOML file.
- **(b) Move scanning behind a channel** owned by the app. More machinery than this step
  warrants.

Going with (a). `set_root` is written by the scan connection before the walk begins.

### Verification

`cargo check --workspace`, then run the GUI: set a library folder, confirm the root
persists across a restart, confirm "Clear Library" empties it.

---

# Step 2 — Analyzer split

Pure core-internal. No public API change beyond `VisData`'s module path.

### New layout

```
core/src/analyzer/
 ├── mod.rs        kira Effect/EffectBuilder glue, VisData, VisReader, wiring
 ├── meter.rs      per-sample peak + RMS (L/R)
 └── spectrum.rs   FFT accumulation, bin map, AGC, smoothing
```

- **`meter.rs`** — `Meter { peak_l, peak_r, rms_sum_l, rms_sum_r, count }` with
  `push(frame)` and `take() -> MeterFrame`. Runs every sample; no block latency.
- **`spectrum.rs`** — owns `buffer`, `buffer_pos`, `fft`, `fft_scratch`, `fft_buffer`,
  `bins`, `window`, `bin_map`, `agc_peak`, `sample_rate`. `push(sample) -> bool`
  (true when a block completed), `bins() -> &[f32; NUM_BINS]`. AGC and smoothing stay
  here, on the audio thread, per the decision above.
- **`mod.rs`** — `AudioAnalyzer` implements `Effect`; `process()` feeds the meter every
  frame and the spectrum every frame, writing `VisData` to the triple buffer when the
  spectrum reports a completed block. `VisData` / `VisReader` / `AudioAnalyzerBuilder`
  keep their current shapes so `backend.rs` is unchanged.

`lib.rs` export changes from `pub use audio_analyzer::VisData` to `pub use analyzer::VisData`.

### Behavior

Intended to be a pure reorganization — identical output. The one deliberate change worth
considering: currently peak/RMS are only *published* when an FFT block completes
(every 2048 samples ≈ 46 ms @ 44.1 kHz), because `analyze()` does the writing. After the
split the meter could publish more often. **Recommend keeping the current publish
cadence** (one write per block) so VU behavior is unchanged and the triple buffer isn't
written at audio rate; revisit only if the meters feel sluggish.

### Verification

`cargo check`, then eyeball the Spectrum and VU Meters panes against the current build —
they should look identical.

---

# Step 3 — Pull Library out of Player

The cross-crate step. Do this last, in one commit, since it breaks call sites.

### Core: `Player` slims

**Removed entirely** (callers use `Library` directly) — `library()`, `create_playlist`,
`rename_playlist`, `delete_playlist`, `add_track_to_playlist`, `remove_track_from_playlist`,
`set_playlist_cover`, `clear_library`.

**Gain a `&Library` parameter:**

```rust
player.play(&library)?;              // start_current on Idle/Finished
player.update(&library)?;            // auto-advance when Finished
player.start_next(&library)?;
player.start_previous(&library)?;
player.queue_library(&library)?;
player.current_track(&library) -> Option<Track>;   // was clone_current_track
```

**Unchanged** (no library needed) — `pause`, `seek`, `set_volume`, `is_playing`,
`position`, `vis_data`, `queue()`, `queue_back`, `queue_front`, `queue_many`,
`queue_many_front`, `clear_queue`, `shuffle_queue`, `remove_from_queue`,
`move_to_queue_front`, `set_loop_mode`, `cycle_loop_mode`, `loop_mode`.

`Library` drops the `pub(crate)` on its playlist mutators and the `forward_queries!`
macro's outputs stay as-is (they are already the right shape) — the macro itself can stay.

### GUI

- **`app.rs`**: add a `library: Library` field alongside `player`; construct both in
  `Default`. Rewrite the ~15 playback call sites to pass `&self.library`. Note
  `Message::Tick` calls `self.player.update()` then iterates panes — the borrow of
  `self.library` must not overlap `&mut self.panes`; pass `&self.library` into
  `pane.update(...)` which takes `&Player, &Library, &mut ArtCache`.
- **`pane_view.rs`**: add `pub library: &'a Library` to `ViewContext`, and add a
  `library: &Library` param to `PaneView::update`. **This is the leverage point** —
  every pane reads the library through one of these two, so the change is mechanical.
- **Panes touching the library** — `collections.rs` (4 sites), `queue.rs` (2 sites),
  `artwork.rs`, `controls.rs`, `track_info.rs` (`clone_current_track` → `current_track(library)`).
  The six panes with `_player: &Player` unused just gain an ignored `_library` param.
- **`pane.rs`**: `Pane::update` and `Pane::view` thread `library` through to the trait.

### TUI

`tui/src/app.rs` gains a `library: Library` field. It calls no library methods today, so
this is two lines.

### Verification

`cargo check --workspace`, then exercise in the GUI: play/pause, next/prev, queue a
track, queue an album, play a playlist, create/rename/delete a playlist, add a track to
a playlist, auto-advance at end of track, media-key controls.

---

---

# Step 4 — Simplify the schema

Do this before the in-memory work: it shrinks what step 5 has to carry into memory,
and both touch the same files.

### Target schema

```sql
tracks(id, path UNIQUE, title, track_artist, album, album_artist,
       genre, year, track_number, disc_number, comment,
       duration, bit_rate, sample_rate, bit_depth, channels,
       missing)

playlists(id, name, cover_track_id)
playlist_tracks(playlist_id, track_id, position)

meta(key, value)
```

Three tables + meta. Down from four tables, six timestamp columns and four indices.
`missing` stays (see 4b) — it becomes a surfaced feature rather than a hidden filter.

### 4a. Drop the `albums` table

It stores `(name, artist, year)` — all taken directly from track tags at
`albums.rs:13-19` — and `get_all_albums` JOINs back to `tracks` anyway to compute
`sample_track_path`. It is a denormalized cache of data that cannot be read without
consulting the source table.

Its only unique column, `id`, is used purely as a GUI HashMap key for artwork
(`collections.rs:104,250,254`). Once albums are derived in memory, a `Vec` index or the
`(name, artist)` pair serves identically.

Removes: the table, `batch_upsert_albums_from_tracks`, the GROUP BY + JOIN, and a class
of staleness bug (albums outliving their tracks).

**Also fixes a real inconsistency.** Three places disagree on album identity today:
- insert keys on `album_artist` (`albums.rs:16`)
- the join matches `t.album_artist IS a.artist` (`albums.rs:42`)
- `get_tracks_by_album` matches `album_artist OR track_artist` (`tracks.rs:151`)

Deriving in Rust forces one definition. **Decide it explicitly** — recommend grouping on
`(album, album_artist.or(track_artist))` so compilations don't shatter into one album per
featured artist.

### 4b. Keep `missing` — and surface it

**Rows are never deleted.** The scan keeps marking absent files `missing = 1`.

Rationale: playlist membership is user-authored data that cannot be re-derived. A drive
left unmounted, a file moved during a reorganization, or an external disk not plugged in
would otherwise cascade through `playlist_tracks` and silently shorten the user's
playlists — damage they may not notice until long after. A stale row costs ~250 bytes;
a deleted one costs information only the user could reconstruct. Keep the row.

This makes `missing` a real feature rather than dead weight, which changes what the rest
of the plan must carry:

- **Keep the column**, and keep `mark_all_missing()` + the upsert reset to 0.
- **The in-memory index (step 5) loads missing tracks too.** Do *not* filter them out at
  load. `Library::tracks()` should expose everything; filtering is a presentation
  decision, so give consumers both:
  - `tracks(&self) -> &[Track]` — everything
  - `available(&self)` — iterator skipping `missing`
- **`WHERE missing = 0` filters come out of the SQL**, because there is only one query
  left after step 5 (`SELECT *` at startup). The distinction moves into the accessors
  above, where it is explicit and cheap.
- **`Track::missing()` gets its first real consumer.** Today it is never called outside
  core.

### 4b-i. UI treatment (deferred, but design for it now)

The GUI work belongs to the later GUI phase, not this refactor. What matters *here* is
not foreclosing it:

- Queue / library / playlist rows for a missing track render dimmed, with an indicator.
- Playback skips missing tracks rather than erroring. Note `Player::play_track`
  (`player.rs:211`) currently silently no-ops when the file will not load — with a
  `missing` flag available it should instead skip to the next playable entry, so a gap
  in the library does not stall the queue.
- Worth considering later: a "N tracks missing" affordance in preferences, and a
  reconcile-by-path-or-tags step so a moved file re-links instead of orphaning.

### 4b-ii. Consequence to accept

Rows accumulate across rescans of a library whose root moved. If that becomes real, the
answer is an explicit user-initiated "remove missing tracks" action — not silent deletion
during a scan.

### 4c. Drop `created_at` / `updated_at`

Six columns across three tables, written on every insert and update, **read by nothing**.
`Playlist.created_at` / `updated_at` are plumbed into the public struct and never
displayed. Deletes the `now()` helper and every `ts` binding in `playlists.rs`.

If "recently added" is wanted later, add one deliberate column to `tracks` then.

### 4d. Drop the indices

- `idx_tracks_path` — redundant; `path UNIQUE` already creates an index. (Note the scan's
  upsert relies on the `path` conflict target, so the UNIQUE constraint itself stays.)
- `idx_tracks_album`, `idx_tracks_artist`, `idx_tracks_missing` — accelerate queries that
  cease to exist after step 5. The only remaining read is one `SELECT *` at startup,
  which is a full scan regardless.

Indices cost write time during scan — the one operation that is actually slow.

### 4e. Batch the scan insert

`tracks.rs:38` executes a fresh `INSERT` per track inside the transaction. Reuse one
prepared statement across the loop (`tx.prepare` once, `execute` per row). This is the
only hot write path; it is where the 375 ms lives.

### Migration

The existing DB has an incompatible schema and there is exactly one known user
(`D:\audio`, 253 tracks, 375 ms to rebuild). **Recommend: drop and recreate on open,
then rescan.** Writing an `ALTER TABLE` path for a 375 ms rebuild is not worth the code.

**Except playlists.** They are user-authored and unrecoverable — the same reasoning that
keeps `missing` rows. If any playlists exist, carry `playlists` and `playlist_tracks`
across rather than dropping them. Since `playlist_tracks.track_id` points at track rows
that a rescan will recreate with **new ids**, re-link by `path` rather than by id:
read the old `(playlist, track.path, position)` triples before the drop, then reinsert
against the fresh track ids after the scan.

Check whether any playlists exist before writing that code — if the answer is none, drop
everything and skip it.

### Verification

Rescan and confirm track/album counts match the pre-change numbers (253 / 25), that
albums group identically, and that the scan is no slower than 375 ms.

---

# Step 5 — In-memory library index

The step the measurements justify. Do it after step 3, when `Library` is already a
standalone type with a clear boundary.

### Core

- `Library::new()` loads once: `SELECT` all tracks → `Vec<Track>`, build `by_id` /
  `by_path` index maps, derive `albums` in Rust, load `playlists`. ~1.3 ms.
- Query methods become in-memory accessors:
  - `track(&self, id) -> Option<&Track>` — `by_id` hit, ~20 ns (was 12.7 µs)
  - `track_by_path(&self, &Path) -> Option<&Track>`
  - `tracks(&self) -> &[Track]`, `albums(&self) -> &[Album]`, `playlists(&self) -> &[Playlist]`
  - `tracks_by_album(..)`, `tracks_in_playlist(..)` — filter/collect over the slice
- Mutations write through to SQLite **and** update the in-memory state, so no
  invalidation protocol is needed. Playlist mutations are the only writes outside scan.
- Album derivation is already gone from SQL in step 4; here it becomes a load-time
  iterator chain over `tracks`. The `MIN(t.path)` cover-pick and
  `COALESCE(cover_track_id, MIN(t.id))` semantics must be preserved exactly, or artwork
  changes which track it picks.
- Delete the dead API listed above and the `forward_queries!` macro.
- Add `PRAGMA wal_checkpoint(TRUNCATE)` on `Database` drop. Currently `library.db` is
  4 KB with a **638 KB WAL** — it is never checkpointed because the connection is never
  cleanly closed.

### GUI

This is where it pays off — the change is mostly **deletion**:

- Remove `cached_tracks`, `cached_albums`, `cached_playlists` (`app.rs:36-38`),
  `ensure_cached_tracks/albums/playlists`, `invalidate_library_cache`,
  `invalidate_playlist_cache` and all their call sites.
- `ViewContext` drops the three `RefCell` fields; panes read `ctx.library.tracks()` etc.
- The sort in `ensure_cached_tracks` (`app.rs:183`) moves into `Library` so it happens
  once at load, not per invalidation. Note `queue_library` (`player.rs:157`) duplicates
  this exact sort — both collapse to one place.
- `queue.rs:36,113` — `query_track_from_id` becomes `library.track(id)` returning a
  borrow, removing the per-frame clone as well as the SQL.

### Verification

Re-run the benchmark shape: confirm startup load is ~1 ms and that queue rendering no
longer scales with SQL. Confirm album/playlist artwork picks the same tracks as before
(the COALESCE/MIN semantics), and that playlists survive a restart.

---

## Sequencing

Steps are ordered so the tree compiles between each. Recommended as five commits:

1. `core: replace global Config with DB-backed library root`
2. `core: split analyzer into meter and spectrum`
3. `core: pull Library out of Player`
4. `core: simplify schema — drop albums table, timestamps, indices`
5. `core: load library into memory, DB becomes persistence-only`

Steps 1, 2 and 4 are core-internal. Steps 3 and 5 touch `gui` broadly.

**Two orderings worth considering:**

- **Merge 3 + 5.** Both rewrite the same GUI call sites (`queue.rs`, `collections.rs`,
  `pane_view.rs`); doing them apart means touching each twice. Merging halves the churn
  at the cost of a bigger commit.
- **Move 4 earlier.** It is independent of 1–3 and immediately shrinks the surface every
  later step carries. Only ordering constraint is that 4 precedes 5.

Recommended if the combined diff stays reviewable: **1, 2, 4, then 3+5 merged.**

## Net effect on the DB layer

| | Before | After |
| --- | --- | --- |
| Tables | 4 | 3 + `meta` |
| Timestamp columns | 6 | 0 |
| Indices | 4 explicit | 0 explicit (1 implicit via UNIQUE) |
| Query methods | 10 (4 dead) | ~0 — in-memory accessors |
| Layers to reach a track | 3 (`Player`→`Library`→`Database`) | 1 |
| `query_track_from_id` | 12.7 µs | ~20 ns |
| Reads at runtime | per-frame SQL | one `SELECT *` at startup |

## Deliberately out of scope

- Switching audio crates (kira stays).
- Gapless / crossfade / preloading the next track — revisit once the structure settles.
- Moving analyzer smoothing to the UI side (considered; rejected in favor of the
  meter/spectrum split).
- The `search_query` / caching layer in the GUI, and everything else GUI-side.
