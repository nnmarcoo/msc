//! The collections pane: albums and playlists as grids of covers, either of
//! which opens in place to show what is on it.
//!
//! Both are drawn by the same code because they are the same thing to a reader:
//! a cover, a name, and an ordered list of tracks. [`Collection`] is that shape,
//! built from an [`Album`] or a [`Playlist`] as the grid is laid out, so a change
//! to how a tile looks or how a panel reads happens once rather than twice. The
//! two differ only in where their tracks come from and in the order they impose,
//! which is what [`Collection::tracks`] resolves.
//!
//! The grid is built from `column!` and `row!` rather than a custom widget, the
//! way [`crate::widgets::track_list`] is. That is affordable here for the reason
//! it is not there: a library has thousands of tracks but tens of albums, and a
//! cell is a real image widget rather than a strip of text. If a collection grows
//! large enough for a frame to cost more than it should, virtualizing this is the
//! same change made there, and [`layout`] is kept a free function so the row and
//! column arithmetic it would need is already separated and tested.
//!
//! Cell size answers to the pane rather than being fixed: [`layout`] fits as many
//! columns of at least [`MIN_CELL`] as the width allows, then divides the width
//! between them, so a wide pane shows more covers rather than larger ones and a
//! narrow one degrades to a single column instead of clipping. The cover is asked
//! for at the size the cell actually draws, so the same album in a small grid and
//! a large one is two entries in the cache rather than one image stretched.
//!
//! An album's cover is its first track's, since [`verse_core::Album`] orders its
//! tracks by disc and track number. A compilation whose tracks carry different
//! art therefore shows the opening track's, which is the same answer a player's
//! album view usually gives. A playlist names its own cover and falls back to its
//! first track, which is [`verse_core::Playlist::cover_track`]'s rule.
//!
//! A tile is the cover and nothing else. Album art is already the name written
//! in the form its owner chose, so a caption repeating it in the theme's font
//! competes with the thing it labels; a grid of covers alone reads as a shelf.
//! Hovering adds two controls over a scrim and still no caption, since what
//! hover is for is the ability to *act* — the name belongs to the panel a click
//! opens, where there is room to set it properly.
//!
//! # Expanding
//!
//! Clicking a tile opens a panel beneath *its row*, holding the collection's
//! cover, its total running time, and its tracks. Beneath the row rather than
//! beside the tile because a panel that displaced its neighbours would reflow the
//! grid, and one row of covers pushed down is the smallest movement that can
//! show something the width of the pane.
//!
//! The panel is keyed on [`Id`], an owned [`AlbumKey`] or playlist id, rather
//! than on the tile's position. Position is what the click already carries, so
//! keying on it would cost nothing to write, but a query typed while a panel is
//! open renumbers every tile: the panel would stay open on whatever album fell
//! into that slot. An id names the same record however the grid is filtered, and
//! a collection that leaves the grid entirely closes its panel by simply not
//! being found. This is the same reasoning that puts ids rather than row indices
//! in [`crate::browsing::Selection`].
//!
//! That the *click* is still positional is not a contradiction. An index is
//! meaningful against exactly one list, and the click resolves against the list
//! the tile was drawn from, in the same frame; what may not survive is holding
//! that index across frames, which is what the panel would be doing.
//!
//! Expansion is pane state rather than app state, since two collections panes
//! must be able to have different panels open — it describes how one pane draws
//! itself and nothing about the music. See [`crate::pane`].
//!
//! # Where the work is
//!
//! The grid draws from the keys [`crate::app`] cached, turned back into albums by
//! [`verse_core::Library::albums_by_key`]. Filtering here instead would walk every
//! track in the library on every frame, since an album is kept by what its tracks
//! match: a few dozen covers would cost a full library scan sixty times a second.
//! Playlists are filtered here, against their names alone, because that is a
//! comparison per playlist rather than per track and there are tens of them.
//!
//! A tile names its collection by position and nothing else, so the ids it would
//! play are gathered in `update` when it is clicked rather than per tile per
//! frame. A `Vec` built for every tile of every frame is a screenful of
//! allocations for a list that is only ever read once, and never on most frames.
//!
//! A tile's number is its position in *that cached list*, which is the same list
//! `update` resolves it against. Which list is the whole point: numbering against
//! the unfiltered library instead meant a click under a query played whichever
//! album sat that far down the full list, since the two lists agree only when
//! nothing is filtered.
//!
//! The expanded panel resolves its tracks from the library each frame rather than
//! holding a copy. A cached `Vec<Track>` is a third thing that can disagree with
//! the library and the grid, and it would need invalidating on rescan, on rating,
//! and on any playlist edit; resolving is a lookup per row of one open panel.
//!
//! Hover is pane state and the overlay is built in `view` only for the tile that
//! is lit, which is how the pane it replaced did it. Keeping the flag in a
//! custom widget's tree state instead was tried and does not work: `update` runs
//! for every event, and the ones carrying no pointer — a redraw, a resize, a key
//! — arrive with an unavailable cursor that is over nothing, so the widget
//! cleared its own hover a frame or two after the pointer set it and the overlay
//! flashed rather than appearing. A `mouse_area` fires only on real crossings,
//! so nothing else can disturb it.
//!
//! The cost is a pane rebuild per tile crossed, which is what the widget was
//! meant to avoid; it buys an overlay that is actually visible. `Unhovered`
//! carries the tile it left rather than clearing unconditionally, because iced
//! delivers an arrival and a departure in layout order rather than in the order
//! they happened, so moving between two tiles can land the old tile's exit after
//! the new tile's entry.

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    Space, button, column, container, image, mouse_area, row, scrollable, stack, svg, text,
};
use iced::{ContentFit, Element, Length};

use verse_core::{Album, AlbumKey, Library, Playlist, Track};

use crate::app::{Act, Message};
use crate::artwork::Cache;
use crate::browsing::{Context, Query};
use crate::layout::PaneId;
use crate::pane::PaneMessage;
use crate::styles::{self, LABEL_FONT_SIZE, PAD};
use crate::widgets::context_menu::{ContextMenu, Entry};

const MIN_CELL: f32 = 120.0;
const MAX_COLUMNS: usize = 12;
const GAP: f32 = PAD * 2.0;

/// The height of an open panel, as a multiple of the cell size, so a panel keeps
/// its proportion to the covers around it rather than being a fixed slab that
/// dwarfs a small grid and is lost in a large one.
const PANEL_ROWS: f32 = 2.0;

const ICON_PLAY: &[u8] = include_bytes!("../../../assets/icons/play.svg");
const ICON_QUEUE: &[u8] = include_bytes!("../../../assets/icons/queue_add.svg");

/// Big enough to hit over a cover. The edit-mode chrome uses 14, but that sits
/// in a bar with a hover target around it, where this is a bare glyph on art.
const CONTROL_ICON: f32 = 18.0;

const TITLE_SIZE: f32 = 13.0;
const ROW_SIZE: f32 = 12.0;
const NUMBER_WIDTH: f32 = 26.0;
const CLOCK_WIDTH: f32 = 44.0;

/// Which collection a panel is open on.
///
/// Owned rather than borrowed because it outlives the frame that made it, and
/// cheap to compare because that is all it is ever used for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Id {
    Album(AlbumKey),
    Playlist(i64),
}

#[derive(Debug, Default)]
pub struct State {
    pub expanded: Option<Id>,
    pub hovered: Option<Id>,
}

impl State {
    pub const EMPTY: Self = Self {
        expanded: None,
        hovered: None,
    };

    fn is_open(&self, id: &Id) -> bool {
        self.expanded.as_ref() == Some(id)
    }

    fn is_hovered(&self, id: &Id) -> bool {
        self.hovered.as_ref() == Some(id)
    }
}

#[derive(Debug, Clone)]
pub enum PanelMessage {
    Toggle(Id),
    Hovered(Id),
    Unhovered(Id),
}

pub fn update(state: &mut State, message: &PanelMessage) {
    match message {
        // Clicking the open collection closes it, so the tile that opened a
        // panel is also what dismisses it and no separate control is needed.
        PanelMessage::Toggle(id) => {
            state.expanded = (!state.is_open(id)).then(|| id.clone());
        }
        PanelMessage::Hovered(id) => state.hovered = Some(id.clone()),
        // Departures are matched against the tile that is actually lit, because
        // iced delivers an arrival and a departure in layout order rather than
        // in the order they happened: moving between two tiles can land the old
        // tile's exit *after* the new tile's entry, and an unconditional clear
        // would blank the highlight that had just been set.
        PanelMessage::Unhovered(id) => {
            if state.is_hovered(id) {
                state.hovered = None;
            }
        }
    }
}

fn hover(pane: PaneId, id: Id) -> Message {
    Message::Pane(pane, PaneMessage::Collections(PanelMessage::Hovered(id)))
}

fn unhover(pane: PaneId, id: Id) -> Message {
    Message::Pane(pane, PaneMessage::Collections(PanelMessage::Unhovered(id)))
}

/// Opening a panel is this pane's own business, so the message carries the
/// [`PaneId`] that owns the state it toggles; two collections panes must be able
/// to have different panels open.
fn toggle(pane: PaneId, id: Id) -> Message {
    Message::Pane(pane, PaneMessage::Collections(PanelMessage::Toggle(id)))
}

/// Which grid a position indexes. The two are numbered separately, so a bare
/// index would name a tile in whichever grid the reader happened to assume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Album,
    Playlist,
}

/// An album or a playlist, reduced to what the grid and the panel draw.
///
/// Borrows rather than owns: it lives for the frame that built it, and copying
/// a name per tile per frame is a screenful of allocations at the tick rate.
#[derive(Clone, Copy)]
struct Collection<'a> {
    id_of: Source<'a>,
    name: &'a str,
    subtitle: Option<&'a str>,
}

#[derive(Clone, Copy)]
enum Source<'a> {
    Album(&'a Album),
    Playlist(&'a Playlist),
}

impl<'a> Collection<'a> {
    fn album(album: &'a Album) -> Self {
        Self {
            id_of: Source::Album(album),
            name: album.name(),
            subtitle: album.artist(),
        }
    }

    fn playlist(playlist: &'a Playlist) -> Self {
        Self {
            id_of: Source::Playlist(playlist),
            name: &playlist.name,
            subtitle: None,
        }
    }

    fn id(&self) -> Id {
        match self.id_of {
            Source::Album(album) => Id::Album(album.key.clone()),
            Source::Playlist(playlist) => Id::Playlist(playlist.id),
        }
    }

    fn kind(&self) -> Kind {
        match self.id_of {
            Source::Album(_) => Kind::Album,
            Source::Playlist(_) => Kind::Playlist,
        }
    }

    /// The tracks on this collection, in the order it imposes: an album by disc
    /// and track number, a playlist by the order it was authored in.
    ///
    /// Missing files are kept rather than filtered, so a playlist reads as the
    /// list its owner made. A track that cannot be played is drawn dimmed by
    /// [`track_row`] instead of silently vanishing, which would make a playlist
    /// look shorter than it is for reasons the pane never explains.
    fn tracks(&self, library: &'a Library) -> Box<dyn Iterator<Item = &'a Track> + 'a> {
        match self.id_of {
            Source::Album(album) => Box::new(library.album_tracks(album)),
            Source::Playlist(playlist) => Box::new(playlist_tracks(library, playlist)),
        }
    }

    /// The track whose art is this collection's cover.
    fn cover(&self, library: &'a Library) -> Option<&'a Track> {
        match self.id_of {
            Source::Album(album) => library.album_tracks(album).next(),
            Source::Playlist(playlist) => playlist.cover_track().and_then(|id| library.track(id)),
        }
    }
}

/// A playlist's tracks, in its own order, skipping ids the library has lost.
///
/// [`Library::playlist_tracks`] would do this by id, but it looks the playlist
/// up again by id on every call; the caller here already holds it.
fn playlist_tracks<'a>(
    library: &'a Library,
    playlist: &'a Playlist,
) -> impl Iterator<Item = &'a Track> {
    playlist
        .track_ids
        .iter()
        .filter_map(move |&id| library.track(id))
}

pub fn view<'a>(
    tracks: Context<'a>,
    visible: &'a [AlbumKey],
    art: &'a Cache,
    state: &'a State,
    pane: PaneId,
) -> Element<'a, Message> {
    let library = tracks.library;
    let query = tracks.query();

    let has_albums = !visible.is_empty();
    let has_playlists = visible_playlists(library, &query).next().is_some();

    if !has_albums && !has_playlists {
        return empty(library, tracks.search);
    }

    // The grids are rebuilt inside the closure rather than captured, because a
    // `responsive` closure is `Fn` and so cannot hand out anything borrowing
    // what it captured. Both are a map over a list the app already filtered.
    iced::widget::responsive(move |size| {
        let albums: Vec<Collection<'a>> = library
            .albums_by_key(visible)
            .into_iter()
            .map(Collection::album)
            .collect();
        let playlists: Vec<Collection<'a>> = visible_playlists(library, &query)
            .map(Collection::playlist)
            .collect();

        let Some(grid) = layout(size.width, albums.len().max(playlists.len())) else {
            return Space::new().into();
        };

        let mut body = column![].spacing(GAP * 2.0).padding(PAD);

        for (label, collections, kind) in [
            ("Albums", &albums, Kind::Album),
            ("Playlists", &playlists, Kind::Playlist),
        ] {
            if collections.is_empty() {
                continue;
            }
            body = body.push(section(
                label,
                collections,
                kind,
                grid,
                library,
                art,
                state,
                pane,
            ));
        }

        scrollable(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .into()
    })
    .into()
}

/// The playlists a query keeps, in the order the grid draws them.
///
/// Public and used by [`crate::app`] as well, because a tile is clicked by its
/// position in this list and the click is resolved against it. Two copies of the
/// rule that drifted would leave a tile playing the playlist beside the one
/// pressed, which is exactly the failure the album path avoids by resolving
/// through the cache the grid drew from.
///
/// Unlike albums this is not cached on the app, because it costs one comparison
/// per playlist rather than a walk of every track: an album is kept when any of
/// its *tracks* matches, which is why filtering those per frame would be a full
/// library scan. Playlists number in the tens and are matched by name alone.
///
/// Name alone, and not contents, because the things differ: an album's name is a
/// tag its tracks carry, so searching for a song on it already finds it, while a
/// playlist's name is the user's own and its tracks have no idea they are on it.
pub fn visible_playlists<'a>(
    library: &'a Library,
    query: &Query,
) -> impl Iterator<Item = &'a Playlist> {
    // The query is cloned into the iterator rather than borrowed, so what comes
    // back carries only the library's lifetime. A caller that builds its query
    // as a local would otherwise have to keep it alive for as long as the
    // collections drawn from it, which is the whole frame. A `Query` is one
    // short string and this is called twice per frame at most.
    let query = query.clone();
    library
        .playlists()
        .iter()
        .filter(move |playlist| query.matches_field(Some(&playlist.name)))
}

#[expect(clippy::too_many_arguments, reason = "one grid's worth of context")]
fn section<'a>(
    label: &'a str,
    collections: &[Collection<'a>],
    kind: Kind,
    grid: Grid,
    library: &'a Library,
    art: &'a Cache,
    state: &'a State,
    pane: PaneId,
) -> Element<'a, Message> {
    let mut section = column![heading(label, collections.len())].spacing(GAP);

    for (line, chunk) in collections.chunks(grid.columns).enumerate() {
        let first = line * grid.columns;
        let cells = chunk.iter().enumerate().map(|(offset, collection)| {
            tile(
                *collection,
                first + offset,
                kind,
                library,
                art,
                grid,
                state,
                pane,
            )
        });

        section = section.push(row(cells).spacing(GAP));

        // The panel follows the row its collection sits in, so at most one row
        // is ever displaced and the tile that opened it stays in view.
        let open = chunk
            .iter()
            .position(|held| state.is_open(&held.id()))
            .map(|offset| (chunk[offset], first + offset));

        if let Some((collection, shown)) = open {
            section = section.push(panel(collection, shown, kind, library, art, grid));
        }
    }

    section.into()
}

fn heading(label: &str, count: usize) -> Element<'_, Message> {
    row![
        text(label).size(LABEL_FONT_SIZE).style(styles::dim_text),
        text(count.to_string())
            .size(LABEL_FONT_SIZE)
            .style(styles::faint_text),
    ]
    .spacing(PAD)
    .align_y(Vertical::Center)
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub columns: usize,
    pub cell: f32,
}

pub fn layout(width: f32, collections: usize) -> Option<Grid> {
    if width <= 0.0 || collections == 0 {
        return None;
    }

    let inner = width - PAD * 2.0;
    if inner < MIN_CELL {
        return Some(Grid {
            columns: 1,
            cell: inner.max(1.0),
        });
    }

    let fits = ((inner + GAP) / (MIN_CELL + GAP)).floor() as usize;
    let columns = fits.clamp(1, MAX_COLUMNS).min(collections);
    let cell = (inner - GAP * (columns - 1) as f32) / columns as f32;

    // Floored to a whole pixel so the cover and the scrim over it land on the
    // same grid. A fractional cell is rounded independently by the `image`,
    // which sizes itself from the source's aspect ratio, and by the plain
    // container the overlay is, so the two edges disagreed by up to a pixel —
    // visible as a dark hairline down one side of some covers and not others,
    // depending on the proportions of the art. Flooring rather than rounding
    // keeps the row inside the pane, which `the_columns_and_gaps_fill_the_pane`
    // checks; the leftover fraction is at most one pixel per column and shows
    // as a slightly wider gap at the end of the row.
    Some(Grid {
        columns,
        cell: cell.floor().max(1.0),
    })
}

#[expect(clippy::too_many_arguments, reason = "one tile's worth of context")]
fn tile<'a>(
    collection: Collection<'a>,
    shown: usize,
    kind: Kind,
    library: &'a Library,
    art: &'a Cache,
    grid: Grid,
    state: &'a State,
    pane: PaneId,
) -> Element<'a, Message> {
    let id = collection.id();
    let face = cover(collection, library, art, grid.cell);

    // The cover is a button so that a click anywhere on it opens the panel, and
    // the overlay stacks over it. The overlay exists in the tree only while this
    // tile is the hovered one: a hidden layer that still laid out would sit over
    // the cover and eat the press meant for it.
    let body = button(face)
        .padding(0)
        .width(Length::Fixed(grid.cell))
        .height(Length::Fixed(grid.cell))
        .style(styles::tile_style)
        .on_press(toggle(pane, id.clone()));

    let mut layers = stack![body]
        .width(Length::Fixed(grid.cell))
        .height(Length::Fixed(grid.cell));

    if state.is_hovered(&id) {
        layers = layers.push(controls(shown, kind));
    }

    let tile = mouse_area(layers)
        .on_enter(hover(pane, id.clone()))
        .on_exit(unhover(pane, id));

    ContextMenu::new(tile, menu(collection, shown, kind, pane)).into()
}

/// The cover, at the size the cell actually draws.
fn cover<'a>(
    collection: Collection<'a>,
    library: &'a Library,
    art: &'a Cache,
    edge: f32,
) -> Element<'a, Message> {
    let handle = collection
        .cover(library)
        .and_then(|track| Some((track.id()?, track.path())))
        .and_then(|(id, path)| art.request(id, path, edge));

    match handle {
        // Clipped by the container rather than sized by the image alone. With
        // `ContentFit::Cover` the image derives its drawn rectangle from the
        // source's aspect ratio and rounds that itself, so a cover that is not
        // exactly square settled a fraction of a pixel away from the box the
        // scrim fills — a dark hairline down one edge of some covers and not
        // others. Fixing the container and clipping to it makes the two layers
        // agree by construction: whatever the image does inside, the tile's
        // bounds are the same rectangle for both.
        //
        // Square, with no radius on the image. A cover and the scrim over it are
        // separate layers, and each would rasterize its own arc: the two differ
        // by a fraction of a pixel along the curve, which reads as a soft or
        // doubled corner wherever they overlap. A square tile has no arc to
        // disagree about, and a shelf of square sleeves is what the grid is
        // imitating anyway. This is the one place the global corner radius is
        // deliberately not read; see [`crate::styles`].
        Some(handle) => container(
            image(handle)
                .content_fit(ContentFit::Cover)
                .width(Length::Fixed(edge))
                .height(Length::Fixed(edge))
                .filter_method(image::FilterMethod::Linear),
        )
        .width(Length::Fixed(edge))
        .height(Length::Fixed(edge))
        .clip(true)
        .into(),
        None => container(Space::new())
            .width(Length::Fixed(edge))
            .height(Length::Fixed(edge))
            .style(styles::cover_placeholder_style)
            .into(),
    }
}

/// What a hovered tile shows: two actions, and nothing else.
///
/// No name and no artist. The cover already says which record this is, in the
/// form its owner chose, so a caption repeating it in the theme's font competes
/// with the thing it labels — the same reason the resting tile is bare. What
/// hover adds is the ability to *act*, which is what a control is for; the name
/// is available in the panel a click opens, where there is room to set it.
///
/// Only play and add-to-queue, with the rest left to the right-click menu,
/// because a control small enough to fit several of over a cover is too small to
/// hit reliably, and these two are what a cover is usually clicked for.
///
/// They sit centred at the foot of the tile, over a scrim that spans the whole
/// cover and fades out towards the top; see [`styles::over_art_style`].
fn controls<'a>(shown: usize, kind: Kind) -> Element<'a, Message> {
    let buttons = row![
        over_art_button(ICON_PLAY, Message::Collection(kind, shown, Act::Play)),
        over_art_button(ICON_QUEUE, Message::Collection(kind, shown, Act::Queue)),
    ]
    .spacing(PAD);

    container(buttons)
        .padding(PAD)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Bottom)
        .clip(true)
        .style(styles::over_art_style)
        .into()
}

/// A control drawn over cover art: white, on the scrim.
fn over_art_button<'a>(bytes: &'static [u8], message: Message) -> Element<'a, Message> {
    button(
        svg(svg::Handle::from_memory(bytes))
            .style(styles::over_art_svg_style)
            .width(Length::Fixed(CONTROL_ICON))
            .height(Length::Fixed(CONTROL_ICON)),
    )
    .on_press(message)
    .padding(PAD)
    .style(styles::over_art_button_style)
    .into()
}

/// A control on a themed surface, which is what a panel's heading is.
fn icon_button<'a>(bytes: &'static [u8], message: Message) -> Element<'a, Message> {
    button(
        svg(svg::Handle::from_memory(bytes))
            .style(styles::svg_style)
            .width(Length::Fixed(CONTROL_ICON))
            .height(Length::Fixed(CONTROL_ICON)),
    )
    .on_press(message)
    .padding(PAD)
    .style(styles::panel_button_style)
    .into()
}

fn menu(collection: Collection<'_>, shown: usize, kind: Kind, pane: PaneId) -> Vec<Entry<Message>> {
    vec![
        Entry::button("Play", Message::Collection(kind, shown, Act::Play)),
        Entry::button("Play next", Message::Collection(kind, shown, Act::Next)),
        Entry::button("Add to queue", Message::Collection(kind, shown, Act::Queue)),
        Entry::Separator,
        Entry::button("Show tracks", toggle(pane, collection.id())),
    ]
}

/// The open collection: its cover, what it is, and its tracks.
fn panel<'a>(
    collection: Collection<'a>,
    shown: usize,
    kind: Kind,
    library: &'a Library,
    art: &'a Cache,
    grid: Grid,
) -> Element<'a, Message> {
    let height = grid.cell * PANEL_ROWS + GAP;
    let edge = height - PAD * 2.0;

    let tracks: Vec<&'a Track> = collection.tracks(library).collect();
    let total: f32 = tracks.iter().map(|track| track.duration()).sum();

    // Play leads the heading rather than trailing it. Pushed to the far right by
    // a filler it sat a whole panel's width from the title it acts on, which on
    // a wide pane put the control and the thing it plays at opposite ends of the
    // eye's travel. Reading order is what the button belongs to.
    // The heading sits at the tinted end of the panel, on a color taken from
    // the cover rather than one the theme picked, so its text is white for the
    // same reason the text over the art is: no palette entry is reliably legible
    // against an unknown color. The track list further along is on the theme's
    // own background and keeps the theme's own colors.
    let heading = row![
        icon_button(ICON_PLAY, Message::Collection(kind, shown, Act::Play)),
        column![
            text(collection.name)
                .size(TITLE_SIZE)
                .style(styles::over_tint_text)
                .wrapping(text::Wrapping::None),
            text(match collection.subtitle {
                Some(artist) => format!("{artist} · {}", crate::pane::summary(tracks.len(), total)),
                None => crate::pane::summary(tracks.len(), total),
            })
            .size(LABEL_FONT_SIZE)
            .style(styles::over_tint_dim_text)
            .wrapping(text::Wrapping::None),
        ]
        .spacing(2),
    ]
    .align_y(Vertical::Center)
    .spacing(PAD);

    let rows = column(
        tracks
            .iter()
            .enumerate()
            .map(|(index, track)| track_row(track, index, collection.kind())),
    );

    let listing = column![
        heading,
        scrollable(rows)
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            )),
    ]
    .spacing(PAD)
    .width(Length::Fill)
    .height(Length::Fill);

    let body = row![
        container(cover(collection, library, art, edge))
            .width(Length::Fixed(edge))
            .height(Length::Fixed(edge)),
        listing,
    ]
    .spacing(PAD * 2.0)
    .align_y(Vertical::Top);

    // The panel is tinted by the cover it belongs to, which the cache has
    // already named for the art the grid drew. Nothing is requested here: the
    // color arrives with the cover, so a panel is plain for the frame or two
    // before its art resolves and then tints along with it.
    let tint = collection
        .cover(library)
        .and_then(Track::id)
        .and_then(|id| art.color(id));

    container(body)
        .padding(PAD)
        .width(Length::Fill)
        .height(Length::Fixed(height))
        .style(styles::panel_style(tint))
        .into()
}

/// One track in an open panel.
///
/// Numbered by its tag on an album, since that is the number printed on the
/// sleeve and gaps in it are real, and by position in a playlist, where the tag
/// would count against a record the user did not assemble.
fn track_row(track: &Track, index: usize, kind: Kind) -> Element<'_, Message> {
    let number = match kind {
        Kind::Album => track.track_number().unwrap_or(index as u32 + 1),
        Kind::Playlist => index as u32 + 1,
    };

    let missing = track.missing();
    let title = track.title().unwrap_or("Untitled");

    let line = row![
        text(number.to_string())
            .size(ROW_SIZE)
            .style(styles::faint_text)
            .align_x(Horizontal::Right)
            .width(Length::Fixed(NUMBER_WIDTH)),
        text(title)
            .size(ROW_SIZE)
            .style(if missing {
                styles::faint_text
            } else {
                styles::plain_text
            })
            .width(Length::Fill),
        text(clock(track.duration()))
            .size(ROW_SIZE)
            .style(styles::faint_text)
            .align_x(Horizontal::Right)
            .width(Length::Fixed(CLOCK_WIDTH)),
    ]
    .spacing(PAD)
    .align_y(Vertical::Center);

    // A missing file has nothing to play, so its row is inert rather than a
    // button that silently does nothing when pressed.
    let Some(id) = track.id().filter(|_| !missing) else {
        return container(line)
            .padding([PAD, PAD])
            .width(Length::Fill)
            .into();
    };

    button(line)
        .on_press(Message::PlayTrack(id))
        .padding([PAD, PAD])
        .width(Length::Fill)
        .style(styles::listing_row_style)
        .into()
}

fn clock(seconds: f32) -> String {
    let total = if seconds.is_finite() && seconds > 0.0 {
        seconds.round() as u64
    } else {
        0
    };
    format!("{}:{:02}", total / 60, total % 60)
}

/// The two empty states, which are different messages.
///
/// A library with no collections at all is not a search that matched none of
/// them, and saying "Nothing here" under a query would read as a library that
/// had lost them.
fn empty<'a>(library: &Library, search: &str) -> Element<'a, Message> {
    let message = if search.trim().is_empty() {
        if library.is_empty() {
            "No library".to_owned()
        } else {
            "No albums or playlists".to_owned()
        }
    } else {
        format!("No results for \u{201c}{}\u{201d}", search.trim())
    };

    container(text(message).size(14).style(styles::dim_text))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The number each tile carries, which is its index into the visible list
    /// `update` resolves against. The grid is built from `chunks`, so the number
    /// is reconstructed from the row and the position within it; getting that
    /// arithmetic wrong plays a different album than the one clicked, and only
    /// the covers on the first row would look right.
    fn numbering(collections: usize, columns: usize) -> Vec<usize> {
        (0..collections)
            .collect::<Vec<usize>>()
            .chunks(columns)
            .enumerate()
            .flat_map(|(line, chunk)| {
                let first = line * columns;
                (0..chunk.len()).map(move |offset| first + offset)
            })
            .collect()
    }

    fn album_id(name: &str) -> Id {
        Id::Album(AlbumKey {
            name: name.to_owned(),
            artist: None,
        })
    }

    #[test]
    fn a_tile_is_numbered_by_its_place_in_the_visible_list() {
        for columns in 1..6 {
            assert_eq!(
                numbering(11, columns),
                (0..11).collect::<Vec<usize>>(),
                "in {columns} columns a tile would play the wrong album"
            );
        }
    }

    /// The bug the numbering replaced, stated as the difference it turns on: a
    /// tile's number indexes the *visible* list, so a filter that drops earlier
    /// albums does not shift what the remaining covers play. Resolving the same
    /// number against the unfiltered library picks a different record.
    #[test]
    fn filtering_does_not_shift_what_a_cover_plays() {
        let library = ["Blue Lines", "Mezzanine", "Protection"];
        let visible: Vec<&str> = library
            .iter()
            .copied()
            .filter(|name| *name != "Blue Lines")
            .collect();

        for (shown, name) in numbering(visible.len(), 2).into_iter().zip(&visible) {
            assert_eq!(visible[shown], *name);
        }

        assert_ne!(
            library[numbering(visible.len(), 2)[0]],
            visible[0],
            "the filter dropped nothing, so this proves nothing"
        );
    }

    #[test]
    fn a_wider_pane_shows_more_covers_rather_than_larger_ones() {
        let narrow = layout(400.0, 50).expect("a pane with width");
        let wide = layout(900.0, 50).expect("a pane with width");

        assert!(
            wide.columns > narrow.columns,
            "widening the pane grew the cells instead of adding columns"
        );
        assert!(wide.cell < narrow.cell * 2.0);
    }

    #[test]
    fn every_cell_is_at_least_the_minimum_until_the_pane_is_smaller() {
        for width in (MIN_CELL as u32 + 11..1600).step_by(7) {
            let grid = layout(width as f32, 100).expect("a pane with width");
            assert!(
                grid.cell >= MIN_CELL - 1.0,
                "a {width}px pane made {}px cells, below the {MIN_CELL}px floor",
                grid.cell
            );
        }
    }

    #[test]
    fn the_columns_and_gaps_fill_the_pane_without_overflowing() {
        for width in (200..1600).step_by(13) {
            let width = width as f32;
            let grid = layout(width, 100).expect("a pane with width");
            let used = grid.cell * grid.columns as f32 + GAP * (grid.columns - 1) as f32;

            assert!(
                used <= width - PAD * 2.0 + 0.01,
                "{} columns of {} overflowed a {width}px pane by {}",
                grid.columns,
                grid.cell,
                used - (width - PAD * 2.0)
            );
        }
    }

    /// The cover and the scrim over it are separate layers of a `stack`, and
    /// each rounds its own rectangle. A fractional cell let them disagree by up
    /// to a pixel, which showed as a dark hairline down one edge of some covers
    /// and not others, depending on the proportions of the art.
    #[test]
    fn a_cell_is_a_whole_number_of_pixels() {
        for width in (200..2000).step_by(7) {
            let grid = layout(width as f32, 100).expect("a pane with width");
            assert!(
                grid.cell.fract() == 0.0,
                "a {width}px pane made {}px cells, so the scrim would sit a \
                 fraction of a pixel off the cover",
                grid.cell
            );
        }
    }

    #[test]
    fn a_pane_too_narrow_for_one_cell_still_draws_a_single_column() {
        let grid = layout(60.0, 10).expect("a pane with width");

        assert_eq!(grid.columns, 1);
        assert!(grid.cell > 0.0, "a cell of {} cannot be drawn", grid.cell);
    }

    #[test]
    fn a_grid_never_has_more_columns_than_it_has_collections() {
        for collections in 1..8 {
            let grid = layout(2000.0, collections).expect("a pane with width");
            assert!(
                grid.columns <= collections,
                "{collections} collections were laid out in {} columns",
                grid.columns
            );
        }
    }

    #[test]
    fn an_empty_or_unsized_pane_lays_out_nothing() {
        assert!(layout(0.0, 10).is_none());
        assert!(layout(-5.0, 10).is_none());
        assert!(layout(800.0, 0).is_none());
    }

    #[test]
    fn a_huge_pane_stops_adding_columns() {
        let grid = layout(20_000.0, 500).expect("a pane with width");
        assert!(
            grid.columns <= MAX_COLUMNS,
            "a very wide pane made {} columns of postage stamps",
            grid.columns
        );
    }

    #[test]
    fn widening_never_takes_a_column_away() {
        let mut last = 0;
        for width in (200..2000).step_by(11) {
            let grid = layout(width as f32, 500).expect("a pane with width");
            assert!(
                grid.columns >= last,
                "widening to {width} dropped from {last} to {} columns",
                grid.columns
            );
            last = grid.columns;
        }
    }

    #[test]
    fn clicking_a_tile_opens_it_and_clicking_it_again_closes_it() {
        let mut state = State::default();
        let id = album_id("Mezzanine");

        update(&mut state, &PanelMessage::Toggle(id.clone()));
        assert!(state.is_open(&id));

        update(&mut state, &PanelMessage::Toggle(id.clone()));
        assert!(
            !state.is_open(&id),
            "the tile that opened the panel could not dismiss it"
        );
    }

    #[test]
    fn opening_a_second_collection_closes_the_first() {
        let mut state = State::default();
        let (first, second) = (album_id("Mezzanine"), album_id("Protection"));

        update(&mut state, &PanelMessage::Toggle(first.clone()));
        update(&mut state, &PanelMessage::Toggle(second.clone()));

        assert!(state.is_open(&second));
        assert!(
            !state.is_open(&first),
            "two panels were open at once, so the grid held two displaced rows"
        );
    }

    #[test]
    fn an_album_and_a_playlist_are_never_the_same_panel() {
        let mut state = State::default();
        update(&mut state, &PanelMessage::Toggle(Id::Playlist(1)));

        assert!(!state.is_open(&album_id("Mezzanine")));
        assert!(state.is_open(&Id::Playlist(1)));
    }

    /// The reason the panel is keyed on an id rather than on the tile's number.
    /// A query typed while a panel is open renumbers the grid, so a positional
    /// key would leave the panel open on whichever album fell into that slot.
    #[test]
    fn a_panel_follows_its_collection_rather_than_its_position() {
        let before = ["Blue Lines", "Mezzanine", "Protection"];
        let after = ["Mezzanine", "Protection"];

        let mut state = State::default();
        update(&mut state, &PanelMessage::Toggle(album_id(before[1])));

        let still_open = after
            .iter()
            .position(|name| state.is_open(&album_id(name)))
            .expect("the album is still in the filtered grid");

        assert_eq!(
            after[still_open], "Mezzanine",
            "the panel moved to the album that took its old slot"
        );
        assert_ne!(
            still_open, 1,
            "the filter shifted nothing, so this proves nothing"
        );
    }

    #[test]
    fn a_collection_filtered_out_of_the_grid_draws_no_panel() {
        let mut state = State::default();
        update(&mut state, &PanelMessage::Toggle(album_id("Blue Lines")));

        let shown = ["Mezzanine", "Protection"];
        assert!(
            !shown.iter().any(|name| state.is_open(&album_id(name))),
            "a panel drew for an album no longer in the grid"
        );
    }

    /// The rule [`visible_playlists`] applies, over a name alone. `Library` has
    /// no test constructor, so the list it filters cannot be built here; what
    /// can be checked is which playlist a query keeps, which is the predicate
    /// the filter is written from.
    fn kept(query: &str, name: &str) -> bool {
        Query::new(query).matches_field(Some(name))
    }

    #[test]
    fn a_playlist_is_kept_by_its_name() {
        assert!(kept("late", "Late Night"));
        assert!(kept("NIGHT", "Late Night"));
        assert!(!kept("morning", "Late Night"));
    }

    #[test]
    fn an_empty_query_keeps_every_playlist() {
        assert!(kept("", "Late Night"));
        assert!(kept("   ", "Late Night"));
    }

    /// Unlike an album, whose tracks are searched, a playlist is matched by its
    /// own name: its tracks have no idea they are on it.
    #[test]
    fn a_playlist_is_not_kept_by_what_is_on_it() {
        assert!(
            !kept("mezzanine", "Late Night"),
            "a playlist matched a query naming a track it holds, which would              mean walking every track on every playlist per frame"
        );
    }

    /// The hover overlay is drawn inside the cell, so its controls must fit the
    /// smallest cell the grid will ever make. Two buttons side by side, each an
    /// icon plus its padding, with the container's own padding around them.
    #[test]
    fn the_hover_controls_fit_the_smallest_cell() {
        let button = CONTROL_ICON + PAD * 2.0;
        let needed = button * 2.0 + PAD + PAD * 2.0;

        assert!(
            needed <= MIN_CELL,
            "{needed}px of controls do not fit a {MIN_CELL}px cell, so a hovered \
             cover would clip its own buttons"
        );
    }

    #[test]
    fn pointing_at_a_tile_lights_it() {
        let mut state = State::default();
        let id = album_id("Mezzanine");

        update(&mut state, &PanelMessage::Hovered(id.clone()));

        assert!(state.is_hovered(&id));
    }

    #[test]
    fn leaving_a_tile_puts_it_out() {
        let mut state = State::default();
        let id = album_id("Mezzanine");

        update(&mut state, &PanelMessage::Hovered(id.clone()));
        update(&mut state, &PanelMessage::Unhovered(id.clone()));

        assert!(state.hovered.is_none());
    }

    /// iced delivers an arrival and a departure in layout order rather than in
    /// the order they happened, so sliding from one cover to the next can land
    /// the old tile's exit *after* the new tile's entry. Clearing on any exit
    /// would blank the highlight that had just been set, and the overlay would
    /// disappear as the pointer moved along a row.
    #[test]
    fn a_late_departure_does_not_blank_the_tile_just_entered() {
        let mut state = State::default();
        let (left, entered) = (album_id("Blue Lines"), album_id("Mezzanine"));

        update(&mut state, &PanelMessage::Hovered(left.clone()));
        update(&mut state, &PanelMessage::Hovered(entered.clone()));
        update(&mut state, &PanelMessage::Unhovered(left));

        assert!(
            state.is_hovered(&entered),
            "the tile the pointer moved onto went dark when the old one reported              leaving, so the overlay flickered off along a row of covers"
        );
    }

    #[test]
    fn hovering_is_independent_of_what_is_expanded() {
        let mut state = State::default();
        let (open, pointed) = (album_id("Mezzanine"), album_id("Protection"));

        update(&mut state, &PanelMessage::Toggle(open.clone()));
        update(&mut state, &PanelMessage::Hovered(pointed.clone()));

        assert!(state.is_open(&open), "pointing elsewhere closed the panel");
        assert!(state.is_hovered(&pointed));
    }

    #[test]
    fn durations_format_as_minutes_and_seconds() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(61.0), "1:01");
        assert_eq!(clock(599.0), "9:59");
        assert_eq!(clock(f32::NAN), "0:00");
    }
}
