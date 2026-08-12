//! The download pane: fetching a record the user can already name.
//!
//! This is a search box and its answers, and deliberately nothing else. There
//! is no landing feed and no "more like this", because the question this pane
//! exists to answer is "get me *that*" — the user arrives already knowing what
//! they want, and anything offered alongside the results is an answer to a
//! question they did not ask. An empty field therefore shows an empty pane and
//! issues no request at all.
//!
//! Results arrive as two kinds and are drawn as two kinds. Albums are a grid of
//! covers, laid out by [`super::collections::layout`] so a remote album and a
//! held one are the same size and shape on screen; tracks are a list beneath a
//! rule, each with its own art. Interleaving them into one list was the first
//! attempt and it read as a wall of text: an album and a song are different
//! things to want, and the eye should not have to read a label to tell which it
//! is looking at.
//!
//! Artwork is the point rather than a decoration. A cover is how a record is
//! recognised, and a grid of them is scannable in a way a column of titles is
//! not. It arrives from [`crate::artwork::Remote`], which asks per URL and
//! answers with whatever is ready, so a frame never blocks on the network and a
//! cover that fails is asked for once rather than once per frame.
//!
//! A cover is captioned rather than left to speak for itself: title, artist and
//! year are always drawn beneath it, and the explicit badge sits inline after
//! the title. Naming them only on hover made the grid unreadable at a glance —
//! the pointer had to visit each tile in turn to learn what any of it was, which
//! is the opposite of what a wall of art is for.
//!
//! Opening an album expands a panel beneath *its row*, exactly as
//! [`super::collections`] does and for the same reason: a panel that replaced
//! the grid would throw away the thing the user was scanning, and one that
//! displaced its neighbours would reflow it. One row of covers pushed down is
//! the smallest movement that can show something the width of the pane, and
//! clicking the same cover again closes it.
//!
//! The panel grows to fit its tracks, up to [`PANEL_ROWS`] rows of covers. A
//! fixed box was the first attempt and it put the common album — twelve tracks,
//! wanting 310px against the 290 two rows allowed — behind a scrollbar nested
//! inside the pane's own, where the wheel does different things a few pixels
//! apart. Sizing to the content removes that for a normal record and keeps the
//! cap so a long one cannot swallow the grid it was opened from.
//!
//! It draws no cover of its own. The tile that opened it is still on screen
//! directly above, so a second larger copy inside the panel spent a third of the
//! width restating what the user had just pressed.
//!
//! Unlike collections, the tracks are not there to be resolved — they arrive
//! over the network — so [`crate::download::Opened`] carries the request through
//! `Loading`, `Ready` and `Failed` and the panel draws each. It is keyed on the
//! album's id rather than the tile's position, so a search landing while a panel
//! is open cannot leave it attached to whatever album fell into that slot.
//!
//! Rows carry the recording's id rather than their index. A row's position
//! changes the moment the query does, and a download that outlives its query —
//! which most do — would otherwise report progress against whatever row later
//! took that position.
//!
//! Nothing here plays a track. A result is not on disk yet, so the only offer is
//! to download it; it becomes playable by entering the library, which is
//! [`verse_core::Library::ingest_many`]'s job once the file lands. Downloading
//! is the only verb in the pane, and a row that is already held says so rather
//! than offering the button twice.

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    Space, button, column, container, image, mouse_area, row, scrollable, stack, svg, text,
};
use iced::{ContentFit, Element, Length};

use verse_core::explore::{Found, FoundAlbum};

use crate::app::Message;
use crate::artwork::Remote;
use crate::download::{Download, Fetcher, Opened, Search, Stage};
use crate::layout::PaneId;
use crate::pane::PaneMessage;
use crate::styles::{self, LABEL_FONT_SIZE, PAD};
use crate::widgets::marquee::marquee;
use crate::widgets::search_bar::SearchBar;
use crate::widgets::spinner::spinner;

const ICON_DOWNLOAD: &[u8] = include_bytes!("../../../assets/icons/queue_add.svg");
const ICON_CHECK: &[u8] = include_bytes!("../../../assets/icons/check.svg");
const ICON_RETRY: &[u8] = include_bytes!("../../../assets/icons/cycle.svg");

const CONTROL_ICON: f32 = 15.0;

const OVER_ART_ICON: f32 = 18.0;

const HEADING_SIZE: f32 = 14.0;
const TITLE_SIZE: f32 = 13.0;
const ROW_SIZE: f32 = 12.5;

const ROW_ART: f32 = 34.0;
const CLOCK_WIDTH: f32 = 40.0;
const STATE_WIDTH: f32 = 34.0;

const SECTION_GAP: f32 = PAD * 3.0;

const GAP: f32 = PAD * 2.0;

const SCROLLBAR: f32 = 10.0;

const PANEL_ROWS: f32 = 2.0;

const NUMBER_WIDTH: f32 = 26.0;

const TILE_TITLE_SIZE: f32 = 12.0;

const SPINNER_SIZE: f32 = 26.0;
const SPINNER_BAR: f32 = 2.5;
const INLINE_SPINNER: f32 = 15.0;

const BADGE_SIZE: f32 = 8.0;
const BADGE_EDGE: f32 = 13.0;

const CAPTION_HEIGHT: f32 = TILE_TITLE_SIZE + LABEL_FONT_SIZE * 2.0 + PAD * 2.0 + 5.0;

const PANEL_ROW_HEIGHT: f32 = ROW_SIZE + PAD * 2.0;

#[derive(Debug, Default)]
pub struct State {
    pub hovered: Option<String>,
}

impl State {
    pub const EMPTY: Self = Self { hovered: None };

    fn is_hovered(&self, id: &str) -> bool {
        self.hovered.as_deref() == Some(id)
    }
}

#[derive(Debug, Clone)]
pub enum PanelMessage {
    Hovered(String),
    Unhovered(String),
}

pub fn update(state: &mut State, message: &PanelMessage) {
    match message {
        PanelMessage::Hovered(id) => state.hovered = Some(id.clone()),
        PanelMessage::Unhovered(id) => {
            if state.is_hovered(id) {
                state.hovered = None;
            }
        }
    }
}

fn hover(pane: PaneId, id: String) -> Message {
    Message::Pane(pane, PaneMessage::Download(PanelMessage::Hovered(id)))
}

fn unhover(pane: PaneId, id: String) -> Message {
    Message::Pane(pane, PaneMessage::Download(PanelMessage::Unhovered(id)))
}

fn heading(label: &str, count: usize) -> Element<'_, Message> {
    row![
        text(label.to_owned())
            .size(LABEL_FONT_SIZE)
            .style(styles::dim_text),
        text(count.to_string())
            .size(LABEL_FONT_SIZE)
            .style(styles::faint_text),
    ]
    .spacing(PAD)
    .align_y(Vertical::Center)
    .into()
}

pub fn view<'a>(
    search: &'a Search,
    state: &'a State,
    remote: &'a Remote,
    pane: PaneId,
    width: f32,
) -> Element<'a, Message> {
    let mut head = column![search_row(search)]
        .spacing(PAD)
        .width(Length::Fill);

    if search.fetcher == Fetcher::Missing {
        head = head.push(notice());
    }

    container(
        column![head, body(search, state, remote, pane, width)]
            .spacing(PAD)
            .height(Length::Fill),
    )
    .padding(PAD * 2.0)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn search_row(search: &Search) -> Element<'_, Message> {
    let bar = SearchBar::new(
        &search.query,
        Message::DownloadQueryChanged,
        Message::DownloadQueryChanged(String::new()),
    )
    .hero()
    .placeholder("Search for music\u{2026}");

    let mut line = row![bar].spacing(PAD).align_y(Vertical::Center);

    if search.is_searching() {
        line = line.push(
            spinner()
                .size(INLINE_SPINNER)
                .bar_height(2.0)
                .style(|theme: &iced::Theme| styles::faint_text(theme).color.unwrap_or_default()),
        );
    }

    line.into()
}

fn body<'a>(
    search: &'a Search,
    state: &'a State,
    remote: &'a Remote,
    pane: PaneId,
    width: f32,
) -> Element<'a, Message> {
    match &search.stage {
        Stage::Idle => hint("Search to download music"),
        Stage::Searching => waiting(),
        Stage::Failed(reason) => failed(reason),
        Stage::Results(results) if results.is_empty() => hint(&format!(
            "Nothing found for \u{201c}{}\u{201d}",
            search.query.trim()
        )),
        Stage::Results(results) => {
            let mut sections = column![].spacing(SECTION_GAP).width(Length::Fill);

            if !results.albums.is_empty() {
                sections = sections.push(albums(
                    "Albums",
                    &results.albums,
                    search,
                    state,
                    remote,
                    pane,
                    width,
                ));
            }

            if !results.tracks.is_empty() {
                sections = sections.push(heading("Songs", results.tracks.len()));
                sections = sections.push(tracks(&results.tracks, search, remote));
            }

            scroll(sections)
        }
    }
}

fn scroll<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .into()
}

fn albums<'a>(
    label: &'a str,
    found: &'a [FoundAlbum],
    search: &'a Search,
    state: &'a State,
    remote: &'a Remote,
    pane: PaneId,
    width: f32,
) -> Element<'a, Message> {
    let Some(grid) = super::collections::layout(width - SCROLLBAR, found.len()) else {
        return Space::new().into();
    };

    let mut section = column![heading(label, found.len())].spacing(GAP);

    for chunk in found.chunks(grid.columns) {
        let cells = chunk
            .iter()
            .map(|album| album_tile(album, search, state, remote, grid.cell, pane));

        section = section.push(row(cells).spacing(GAP));

        if let Some(opened) = search
            .opened
            .as_ref()
            .filter(|opened| chunk.iter().any(|album| album.id == opened.id()))
        {
            section = section.push(panel(opened, search, grid));
        }
    }

    section.into()
}

fn album_tile<'a>(
    album: &'a FoundAlbum,
    search: &'a Search,
    state: &'a State,
    remote: &'a Remote,
    edge: f32,
    pane: PaneId,
) -> Element<'a, Message> {
    let face = button(art(album.cover_url.as_deref(), remote, edge))
        .padding(0)
        .width(Length::Fixed(edge))
        .height(Length::Fixed(edge))
        .style(styles::tile_style)
        .on_press(Message::DownloadOpenAlbum(album.id.clone()));

    let mut layers = stack![face]
        .width(Length::Fixed(edge))
        .height(Length::Fixed(edge));

    if state.is_hovered(&album.id) {
        layers = layers.push(controls(album, search));
    }

    let cover = mouse_area(layers)
        .on_enter(hover(pane, album.id.clone()))
        .on_exit(unhover(pane, album.id.clone()));

    column![cover, caption(album, edge)]
        .spacing(PAD + 1.0)
        .width(Length::Fixed(edge))
        .into()
}

fn caption(album: &FoundAlbum, edge: f32) -> Element<'_, Message> {
    let mut title = row![
        marquee(album.title.as_str())
            .size(TILE_TITLE_SIZE)
            .style(|theme: &iced::Theme| theme.extended_palette().background.base.text),
    ]
    .spacing(PAD)
    .align_y(Vertical::Center);

    if album.explicit {
        title = title.push(explicit_badge());
    }

    column![
        title,
        marquee(credit(album.artist.as_deref()))
            .size(LABEL_FONT_SIZE)
            .style(|theme: &iced::Theme| styles::dim_text(theme).color.unwrap_or_default()),
        text(album.year.map_or_else(String::new, |year| year.to_string()))
            .size(LABEL_FONT_SIZE)
            .style(styles::faint_text),
    ]
    .spacing(2)
    .width(Length::Fixed(edge))
    .clip(true)
    .into()
}

fn explicit_badge<'a>() -> Element<'a, Message> {
    container(
        text("E")
            .size(BADGE_SIZE)
            .style(styles::dim_text)
            .align_x(Horizontal::Center),
    )
    .width(Length::Fixed(BADGE_EDGE))
    .height(Length::Fixed(BADGE_EDGE))
    .align_x(Horizontal::Center)
    .align_y(Vertical::Center)
    .style(styles::explicit_badge_style)
    .into()
}

fn controls<'a>(album: &'a FoundAlbum, search: &'a Search) -> Element<'a, Message> {
    let buttons = row![over_art_button(
        ICON_DOWNLOAD,
        search
            .fetcher
            .can_download()
            .then(|| Message::DownloadAlbum(album.id.clone())),
    )]
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

fn over_art_button<'a>(bytes: &'static [u8], message: Option<Message>) -> Element<'a, Message> {
    button(
        svg(svg::Handle::from_memory(bytes))
            .style(styles::over_art_svg_style)
            .width(Length::Fixed(OVER_ART_ICON))
            .height(Length::Fixed(OVER_ART_ICON)),
    )
    .on_press_maybe(message)
    .padding(PAD)
    .style(styles::over_art_button_style)
    .into()
}

fn panel<'a>(
    opened: &'a Opened,
    search: &'a Search,
    grid: super::collections::Grid,
) -> Element<'a, Message> {
    let floor = grid.cell + GAP;
    let ceiling = grid.cell * PANEL_ROWS + GAP;

    let height = match opened.album() {
        Some(album) => wanted_height(album.tracks.len()).clamp(floor, ceiling.max(floor)),
        None => floor,
    };

    let body: Element<'a, Message> = match opened {
        Opened::Loading(_) => {
            container(spinner().size(SPINNER_SIZE).bar_height(SPINNER_BAR).style(
                |theme: &iced::Theme| styles::over_tint_dim_text(theme).color.unwrap_or_default(),
            ))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        }
        Opened::Failed(_, reason) => container(
            text(reason.clone())
                .size(ROW_SIZE)
                .style(styles::over_tint_dim_text),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into(),
        Opened::Ready(album) => {
            let heading = row![
                icon_button(
                    ICON_DOWNLOAD,
                    search
                        .fetcher
                        .can_download()
                        .then(|| Message::DownloadAlbum(album.id.clone())),
                ),
                column![
                    text(album.title.clone())
                        .size(TITLE_SIZE)
                        .style(styles::over_tint_text)
                        .wrapping(text::Wrapping::None),
                    text(format!(
                        "{} \u{00b7} {} songs",
                        credit(album.artist.as_deref()),
                        album.tracks.len()
                    ))
                    .size(LABEL_FONT_SIZE)
                    .style(styles::over_tint_dim_text)
                    .wrapping(text::Wrapping::None),
                ]
                .spacing(2),
            ]
            .align_y(Vertical::Center)
            .spacing(PAD);

            let rows = column(
                album
                    .tracks
                    .iter()
                    .enumerate()
                    .map(|(index, track)| panel_row(track, index, search)),
            );

            column![
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
            .height(Length::Fill)
            .into()
        }
    };

    container(body)
        .padding(PAD)
        .width(Length::Fill)
        .height(Length::Fixed(height))
        .style(styles::panel_style(None))
        .into()
}

fn wanted_height(tracks: usize) -> f32 {
    let heading = CONTROL_ICON + PAD * 2.0;
    let rows = tracks as f32 * PANEL_ROW_HEIGHT;

    heading + rows + PAD * 3.0
}

fn panel_row<'a>(track: &'a Found, index: usize, search: &'a Search) -> Element<'a, Message> {
    let line = row![
        text((index + 1).to_string())
            .size(ROW_SIZE)
            .style(styles::over_tint_dim_text)
            .align_x(Horizontal::Right)
            .width(Length::Fixed(NUMBER_WIDTH)),
        text(track.title.clone())
            .size(ROW_SIZE)
            .style(styles::over_tint_text)
            .wrapping(text::Wrapping::None)
            .width(Length::Fill),
        text(clock(track.duration))
            .size(ROW_SIZE)
            .style(styles::over_tint_dim_text)
            .align_x(Horizontal::Right)
            .width(Length::Fixed(CLOCK_WIDTH)),
        panel_state(track, search),
    ]
    .spacing(PAD)
    .align_y(Vertical::Center);

    container(line)
        .padding([PAD, PAD])
        .width(Length::Fill)
        .into()
}

fn panel_state<'a>(track: &'a Found, search: &'a Search) -> Element<'a, Message> {
    let label = |body: String| {
        text(body)
            .size(LABEL_FONT_SIZE)
            .style(styles::over_tint_dim_text)
            .align_x(Horizontal::Right)
            .width(Length::Fixed(STATE_WIDTH))
            .into()
    };

    match search.downloads.get(&track.id) {
        Some(Download::Running(fraction)) => label(format!("{:.0}%", fraction * 100.0)),
        Some(Download::Queued) => label("\u{2026}".to_owned()),
        Some(Download::Failed(_)) => container(icon_button(
            ICON_RETRY,
            search
                .fetcher
                .can_download()
                .then(|| Message::DownloadOne(track.id.clone())),
        ))
        .width(Length::Fixed(STATE_WIDTH))
        .align_x(Horizontal::Right)
        .into(),
        Some(Download::Done(_)) => label("Saved".to_owned()),
        None if search.held.holds(&track.id) => label("Saved".to_owned()),
        None => container(icon_button(
            ICON_DOWNLOAD,
            search
                .fetcher
                .can_download()
                .then(|| Message::DownloadOne(track.id.clone())),
        ))
        .width(Length::Fixed(STATE_WIDTH))
        .align_x(Horizontal::Right)
        .into(),
    }
}

fn icon_button<'a>(bytes: &'static [u8], message: Option<Message>) -> Element<'a, Message> {
    button(
        svg(svg::Handle::from_memory(bytes))
            .style(styles::over_art_svg_style)
            .width(Length::Fixed(CONTROL_ICON))
            .height(Length::Fixed(CONTROL_ICON)),
    )
    .on_press_maybe(message)
    .padding(PAD)
    .style(styles::panel_button_style)
    .into()
}

fn tracks<'a>(
    found: &'a [Found],
    search: &'a Search,
    remote: &'a Remote,
) -> Element<'a, Message> {
    column(found.iter().map(|track| entry(track, search, remote)))
        .width(Length::Fill)
        .into()
}

fn entry<'a>(track: &'a Found, search: &'a Search, remote: &'a Remote) -> Element<'a, Message> {
    let lines = column![
        text(track.title.clone())
            .size(ROW_SIZE)
            .style(styles::plain_text)
            .wrapping(text::Wrapping::None),
        text(credit(track.artist.as_deref()))
            .size(LABEL_FONT_SIZE)
            .style(styles::dim_text)
            .wrapping(text::Wrapping::None),
    ]
    .spacing(1)
    .width(Length::Fill);

    let line = row![
        art(track.cover_url.as_deref(), remote, ROW_ART),
        lines,
        text(clock(track.duration))
            .size(LABEL_FONT_SIZE)
            .style(styles::faint_text)
            .align_x(Horizontal::Right)
            .width(Length::Fixed(CLOCK_WIDTH)),
        download_state(track, search),
    ]
    .spacing(PAD * 2.0)
    .align_y(Vertical::Center);

    container(line)
        .padding([PAD, PAD])
        .width(Length::Fill)
        .into()
}

fn art<'a>(url: Option<&str>, remote: &'a Remote, edge: f32) -> Element<'a, Message> {
    let handle = url.and_then(|url| {
        remote.request(&verse_core::explore::cover_at_size(
            url,
            edge.round() as u32,
        ))
    });

    match handle {
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

fn download_state<'a>(track: &'a Found, search: &'a Search) -> Element<'a, Message> {
    let slot = |content: Element<'a, Message>| {
        container(content)
            .width(Length::Fixed(STATE_WIDTH))
            .align_x(Horizontal::Center)
            .into()
    };

    match search.downloads.get(&track.id) {
        Some(Download::Running(fraction)) => slot(
            text(format!("{:.0}%", fraction * 100.0))
                .size(LABEL_FONT_SIZE)
                .style(styles::dim_text)
                .into(),
        ),
        Some(Download::Queued) => slot(
            text("\u{2026}")
                .size(LABEL_FONT_SIZE)
                .style(styles::faint_text)
                .into(),
        ),
        Some(Download::Failed(_)) => slot(retry_button(track, search)),
        Some(Download::Done(_)) => slot(icon(ICON_CHECK)),
        None if search.held.holds(&track.id) => slot(icon(ICON_CHECK)),
        None => slot(download_button(track, search)),
    }
}

fn icon<'a>(bytes: &'static [u8]) -> Element<'a, Message> {
    svg(svg::Handle::from_memory(bytes))
        .style(styles::faint_svg_style)
        .width(Length::Fixed(CONTROL_ICON))
        .height(Length::Fixed(CONTROL_ICON))
        .into()
}

fn retry_button<'a>(track: &'a Found, search: &'a Search) -> Element<'a, Message> {
    let press = search
        .fetcher
        .can_download()
        .then(|| Message::DownloadOne(track.id.clone()));

    button(
        svg(svg::Handle::from_memory(ICON_RETRY))
            .style(styles::svg_style)
            .width(Length::Fixed(CONTROL_ICON))
            .height(Length::Fixed(CONTROL_ICON)),
    )
    .on_press_maybe(press)
    .padding(PAD)
    .style(styles::listing_row_style)
    .into()
}

fn download_button<'a>(track: &'a Found, search: &'a Search) -> Element<'a, Message> {
    let press = search
        .fetcher
        .can_download()
        .then(|| Message::DownloadOne(track.id.clone()));

    button(
        svg(svg::Handle::from_memory(ICON_DOWNLOAD))
            .style(styles::svg_style)
            .width(Length::Fixed(CONTROL_ICON))
            .height(Length::Fixed(CONTROL_ICON)),
    )
    .on_press_maybe(press)
    .padding(PAD)
    .style(styles::listing_row_style)
    .into()
}

fn credit(artist: Option<&str>) -> String {
    artist.unwrap_or("Unknown artist").to_owned()
}

fn notice<'a>() -> Element<'a, Message> {
    container(
        text("yt-dlp not found \u{2014} run scripts/setup-explore.ps1 to enable downloads")
            .size(LABEL_FONT_SIZE)
            .style(styles::dim_text),
    )
    .width(Length::Fill)
    .into()
}

fn hint<'a>(label: &str) -> Element<'a, Message> {
    container(
        text(label.to_owned())
            .size(TITLE_SIZE)
            .style(styles::dim_text),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn waiting<'a>() -> Element<'a, Message> {
    container(
        spinner()
            .size(SPINNER_SIZE)
            .bar_height(SPINNER_BAR)
            .style(|theme: &iced::Theme| styles::dim_text(theme).color.unwrap_or_default()),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn failed<'a>(reason: &str) -> Element<'a, Message> {
    let body = column![
        text("Search failed")
            .size(HEADING_SIZE)
            .style(styles::plain_text),
        text(reason.to_owned())
            .size(LABEL_FONT_SIZE)
            .style(styles::dim_text),
        button(text("Try again").size(TITLE_SIZE))
            .on_press(Message::DownloadRetry)
            .padding([PAD, PAD * 2.0])
            .style(styles::listing_row_style),
    ]
    .spacing(PAD * 2.0)
    .align_x(iced::Center);

    container(body)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn clock(seconds: Option<u32>) -> String {
    match seconds {
        Some(total) => format!("{}:{:02}", total / 60, total % 60),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::{Download, Fetcher, Results};

    fn found(id: &str) -> Found {
        Found {
            id: id.to_owned(),
            title: "A Song".to_owned(),
            artist: Some("An Artist".to_owned()),
            album: Some("An Album".to_owned()),
            album_id: Some("MPRE1".to_owned()),
            duration: Some(200),
            cover_url: Some("https://host/art=w60-h60-l90-rj".to_owned()),
            explicit: false,
        }
    }

    fn album() -> FoundAlbum {
        FoundAlbum {
            release: verse_core::explore::Release::default(),
            id: "MPRE1".to_owned(),
            title: "An Album".to_owned(),
            artist: Some("An Artist".to_owned()),
            year: Some(2007),
            cover_url: Some("https://host/art=w60-h60-l90-rj".to_owned()),
            explicit: false,
            tracks: vec![found("a"), found("b")],
        }
    }

    fn with(stage: Stage) -> Search {
        let mut search = Search::default();
        search.stage = stage;
        search.fetcher = Fetcher::Ready;
        search
    }

    fn draw(search: &Search) {
        let remote = Remote::new();
        let state = State::default();
        let _ = view(search, &state, &remote, crate::layout::PaneId(0), 800.0);
    }

    #[test]
    fn every_stage_draws_something() {
        let stages = [
            Stage::Idle,
            Stage::Searching,
            Stage::Failed("offline".to_owned()),
            Stage::Results(Box::default()),
            Stage::Results(Box::new(Results {
                albums: vec![album()],
                tracks: vec![found("a")],
            })),
        ];

        for stage in stages {
            draw(&with(stage));
        }
    }

    #[test]
    fn an_empty_field_shows_a_prompt_rather_than_a_feed() {
        let search = with(Stage::Idle);

        assert!(search.stage.albums().is_empty());
        assert!(
            search.stage.tracks().is_empty(),
            "the resting pane offered music nobody asked for"
        );
        draw(&search);
    }

    #[test]
    fn a_row_in_every_download_state_draws() {
        let states = [
            Download::Queued,
            Download::Running(0.42),
            Download::Done(7),
            Download::Failed("nope".to_owned()),
        ];

        for state in states {
            let mut search = with(Stage::Results(Box::new(Results {
                albums: Vec::new(),
                tracks: vec![found("a")],
            })));
            search.downloads.set("a", state);
            draw(&search);
        }
    }

    #[test]
    fn albums_and_tracks_are_separate_sections() {
        let stage = Stage::Results(Box::new(Results {
            albums: vec![album()],
            tracks: vec![found("a"), found("b")],
        }));

        assert_eq!(stage.albums().len(), 1);
        assert_eq!(stage.tracks().len(), 2);
    }

    const _: () = assert!(
        CAPTION_HEIGHT > TILE_TITLE_SIZE + LABEL_FONT_SIZE * 2.0,
        "a caption must reserve room for all three of its lines"
    );

    #[test]
    fn a_short_album_asks_for_less_room_than_a_long_one() {
        assert!(
            wanted_height(4) < wanted_height(12),
            "the panel ignored how many tracks it had to show"
        );
    }

    #[test]
    fn a_long_album_cannot_push_the_grid_off_screen() {
        let grid = super::super::collections::layout(900.0, 12).expect("a grid");
        let ceiling = grid.cell * PANEL_ROWS + GAP;

        assert!(
            wanted_height(30) > ceiling,
            "a thirty-track album fit inside the cap, so nothing is being bounded"
        );

        let drawn = wanted_height(30).clamp(grid.cell + GAP, ceiling);

        assert!(
            (drawn - ceiling).abs() < f32::EPSILON,
            "the panel drew {drawn} rather than stopping at {ceiling}"
        );
    }

    #[test]
    fn a_short_album_is_not_padded_out_to_the_cap() {
        let grid = super::super::collections::layout(900.0, 12).expect("a grid");
        let floor = grid.cell + GAP;
        let ceiling = grid.cell * PANEL_ROWS + GAP;

        let drawn = wanted_height(3).clamp(floor, ceiling);

        assert!(
            drawn < ceiling,
            "a three-track album reserved the full {ceiling} the cap allows"
        );
    }

    #[test]
    fn a_grid_fits_inside_the_pane_it_is_drawn_in() {
        for width in [200.0_f32, 420.0, 900.0, 1600.0] {
            let inner = width - PAD * 4.0 - SCROLLBAR;
            let grid = super::super::collections::layout(inner, 12).expect("a grid");
            let used = grid.cell * grid.columns as f32 + PAD * 2.0 * (grid.columns - 1) as f32;

            assert!(
                used <= inner + 1.0,
                "at {width}px the grid wants {used:.1}px of {inner:.1}px"
            );
        }
    }

    #[test]
    fn hovering_one_tile_does_not_light_another() {
        let mut state = State::default();

        update(&mut state, &PanelMessage::Hovered("a".to_owned()));
        assert!(state.is_hovered("a"));
        assert!(!state.is_hovered("b"));

        update(&mut state, &PanelMessage::Hovered("b".to_owned()));
        assert!(state.is_hovered("b"));
        assert!(!state.is_hovered("a"));
    }

    #[test]
    fn leaving_a_tile_that_is_no_longer_hovered_keeps_the_current_one() {
        let mut state = State::default();

        update(&mut state, &PanelMessage::Hovered("b".to_owned()));
        update(&mut state, &PanelMessage::Unhovered("a".to_owned()));

        assert!(
            state.is_hovered("b"),
            "a late exit from the tile the cursor already left must not clear the new one"
        );
    }

    #[test]
    fn a_row_asks_for_art_larger_than_it_draws() {
        let sized =
            verse_core::explore::cover_at_size("https://host/a=w544-h544-l90-rj", ROW_ART as u32);

        assert!(
            !sized.ends_with(&format!("=w{}-h{}-l90-rj", ROW_ART as u32, ROW_ART as u32)),
            "art served at exactly its drawn size has no pixels spare and reads soft: {sized}"
        );
    }

    #[test]
    fn a_row_draws_its_download_state_whatever_it_is() {
        let mut search = with(Stage::Results(Box::new(Results {
            albums: Vec::new(),
            tracks: vec![found("a")],
        })));

        for state in [
            None,
            Some(Download::Running(0.4)),
            Some(Download::Done(7)),
            Some(Download::Failed("nope".to_owned())),
        ] {
            if let Some(state) = state {
                search.downloads.set("a", state);
            }
            let _ = download_state(&found("a"), &search);
        }
    }

    #[test]
    fn a_pane_without_the_fetcher_still_draws_its_results() {
        let mut search = with(Stage::Results(Box::new(Results {
            albums: vec![album()],
            tracks: vec![found("a")],
        })));
        search.fetcher = Fetcher::Missing;
        search.held.rebuild([found("a")].iter(), |_| true);

        draw(&search);
        let _ = download_state(&found("a"), &search);
    }

    #[test]
    fn a_duration_reads_as_minutes_and_seconds() {
        assert_eq!(clock(Some(200)), "3:20");
        assert_eq!(clock(Some(59)), "0:59");
    }

    #[test]
    fn a_track_of_unknown_length_shows_no_clock() {
        assert_eq!(clock(None), "");
    }

    #[test]
    fn a_missing_artist_still_reads_as_something() {
        assert_eq!(credit(None), "Unknown artist");
        assert_eq!(credit(Some("Someone")), "Someone");
    }
}
