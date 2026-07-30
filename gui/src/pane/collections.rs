//! The collections pane: albums as a grid of covers.
//!
//! Playlists will live here too; only albums are drawn for now, so the pane is
//! named for what it will hold rather than for what it currently shows.
//!
//! The grid is built from `column!` and `row!` rather than a custom widget, the
//! way [`crate::widgets::track_list`] is. That is affordable here for the reason
//! it is not there: a library has thousands of tracks but tens of albums, and a
//! cell is a real image widget rather than a strip of text. If a collection grows
//! large enough for a frame to cost more than it should, virtualising this is the
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
//! album view usually gives.
//!
//! A tile is the cover and nothing else. Album art is already the name written
//! in the form its owner chose, so a caption repeating it in the theme's font
//! competes with the thing it labels; a grid of covers alone reads as a shelf.
//! What is lost is the untagged album and the one whose art is missing, which
//! show as bare placeholders — a hover label is the answer there, when the pane
//! grows the interaction to hang it on.
//!
//! The grid is filtered by the search pane, so one field narrows the covers the
//! same way it narrows a track list; [`crate::browsing`] holds the rule for
//! which albums a query keeps. The two empty states are different messages: a
//! library with no albums at all is not a search that matched none of them, and
//! saying "No albums" under a query would read as a library that had lost them.
//!
//! A tile names the album by its key and nothing else, so the ids it would play
//! are gathered in `update` when it is clicked rather than per tile per frame. A
//! `Vec` built for every tile of every frame is a screenful of allocations for a
//! list that is only ever read once, and never on most frames.
//!
//! The grid draws from the keys [`crate::app`] cached, turned back into albums by
//! [`verse_core::Library::albums_by_key`]. Filtering here instead would walk every
//! track in the library on every frame, since an album is kept by what its tracks
//! match: a few dozen covers would cost a full library scan sixty times a second.
//!
//! A tile's number is its position in *that cached list*, which is the same list
//! `update` resolves it against. Which list is the whole point: numbering against
//! the unfiltered library instead meant a click under a query played whichever
//! album sat that far down the full list, since the two lists agree only when
//! nothing is filtered. Carrying the [`verse_core::AlbumKey`] would also be
//! unambiguous, but a key is two `String`s and a tile builds its message every
//! frame, so a screenful of covers cloned a screenful of strings sixty times a
//! second to name something a `usize` already names exactly.
//!
//! The tile is a plain `button` rather than a `mouse_area` wrapping one. A button
//! with no `on_press` reports `Status::Disabled` and never `Hovered`, so the
//! earlier arrangement silently had no hover highlight at all: the press was
//! handled by the outer widget while the inner one styled itself as dead.

use iced::widget::{Space, button, column, container, image, row, scrollable, text};
use iced::{ContentFit, Element, Length};

use verse_core::{Album, AlbumKey};

use crate::app::Message;
use crate::artwork::Cache;
use crate::browsing::Context;
use crate::styles::{self, PAD};

const MIN_CELL: f32 = 120.0;
const MAX_COLUMNS: usize = 12;
const GAP: f32 = PAD * 2.0;

pub fn view<'a>(
    tracks: Context<'a>,
    visible: &'a [AlbumKey],
    art: &'a Cache,
) -> Element<'a, Message> {
    if tracks.library.albums().is_empty() {
        return empty("No albums");
    }

    let albums: Vec<&'a Album> = tracks.library.albums_by_key(visible);
    if albums.is_empty() {
        return empty("No matching albums");
    }

    iced::widget::responsive(move |size| {
        let Some(grid) = layout(size.width, albums.len()) else {
            return Space::new().into();
        };

        let rows = albums
            .chunks(grid.columns)
            .enumerate()
            .map(|(line, chunk)| {
                let first = line * grid.columns;
                let cells = chunk
                    .iter()
                    .enumerate()
                    .map(|(offset, album)| tile(album, first + offset, tracks, art, grid.cell));

                row(cells).spacing(GAP).into()
            });

        scrollable(column(rows).spacing(GAP).padding(PAD))
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .into()
    })
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub columns: usize,
    pub cell: f32,
}

pub fn layout(width: f32, albums: usize) -> Option<Grid> {
    if width <= 0.0 || albums == 0 {
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
    let columns = fits.clamp(1, MAX_COLUMNS).min(albums);
    let cell = (inner - GAP * (columns - 1) as f32) / columns as f32;

    Some(Grid {
        columns,
        cell: cell.max(1.0),
    })
}

fn tile<'a>(
    album: &'a Album,
    shown: usize,
    tracks: Context<'a>,
    art: &'a Cache,
    cell: f32,
) -> Element<'a, Message> {
    let opener = tracks.library.album_tracks(album).next();

    let cover = opener
        .and_then(|track| Some((track.id()?, track.path())))
        .and_then(|(id, path)| art.request(id, path, cell));

    let face: Element<'a, Message> = match cover {
        Some(handle) => image(handle)
            .content_fit(ContentFit::Cover)
            .width(Length::Fixed(cell))
            .height(Length::Fixed(cell))
            .filter_method(image::FilterMethod::Linear)
            .into(),
        None => container(Space::new())
            .width(Length::Fixed(cell))
            .height(Length::Fixed(cell))
            .style(styles::artwork_placeholder_style)
            .into(),
    };

    button(face)
        .padding(0)
        .style(styles::tile_style)
        .on_press(Message::PlayAlbum(shown))
        .into()
}

fn empty(message: &str) -> Element<'_, Message> {
    container(text(message).size(14).style(|theme: &iced::Theme| {
        text::Style {
            color: Some(
                theme
                    .extended_palette()
                    .background
                    .base
                    .text
                    .scale_alpha(0.6),
            ),
        }
    }))
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
    fn numbering(albums: usize, columns: usize) -> Vec<usize> {
        (0..albums)
            .collect::<Vec<usize>>()
            .chunks(columns)
            .enumerate()
            .flat_map(|(line, chunk)| {
                let first = line * columns;
                (0..chunk.len()).map(move |offset| first + offset)
            })
            .collect()
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

    #[test]
    fn a_pane_too_narrow_for_one_cell_still_draws_a_single_column() {
        let grid = layout(60.0, 10).expect("a pane with width");

        assert_eq!(grid.columns, 1);
        assert!(grid.cell > 0.0, "a cell of {} cannot be drawn", grid.cell);
    }

    #[test]
    fn a_grid_never_has_more_columns_than_it_has_albums() {
        for albums in 1..8 {
            let grid = layout(2000.0, albums).expect("a pane with width");
            assert!(
                grid.columns <= albums,
                "{albums} albums were laid out in {} columns",
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
}
