//! Turning Innertube renderers into [`Found`] and [`FoundAlbum`].
//!
//! This is the file YouTube breaks. Everything here reads defensively and
//! answers `None` rather than faulting, so a reshaped field costs one row and
//! not the whole response: eighteen results of twenty is a far better failure
//! than an error, and it degrades in a way a user can still work with.
//!
//! The fixtures under `tests/fixtures` are real trimmed responses, so a
//! reshape shows up as a named failing test rather than an empty pane.
//!
//! Four shapes here are not guessable from the JSON and were each established
//! against the live API:
//!
//! A row's own thumbnail is a 60px channel avatar, not album art. Cover art
//! comes from the album header, and [`upscale_cover`] rewrites the `=w…` suffix
//! Google's image host uses for sizing. Embedding the row thumbnail would put a
//! 60px avatar in every downloaded file.
//!
//! An album header carries two thumbnails: `thumbnail` is the sleeve and
//! `straplineThumbnail` is a photograph of the artist. They are told apart only
//! by which key they hang from, so cover art is read at a named path rather than
//! by searching the header for the first `thumbnails` array — which finds the
//! band photo and embeds it into every track on the album.
//!
//! A byline reads "Artist • Album • 4:51", but its parts are identified by
//! their `browseEndpoint` rather than their position: singles carry no album,
//! some rows lead with a "Song" label, and splitting on the separator puts the
//! album in a different slot depending on which. The endpoint says what a run
//! *is*, so a missing album shifts nothing.
//!
//! A byline arrives in three containers. A search row wraps its runs in a
//! flex-column renderer, an album row keeps its duration in a *fixed*-column one
//! and leaves the byline empty, and a radio row is itself the container with
//! `runs` directly on it. [`column_runs`] tries each, which is what lets one
//! reader serve all three endpoints.

use serde_json::Value;

use super::nav;
use crate::explore::{Found, FoundAlbum};

const ALBUM_PAGE: &str = "MUSIC_PAGE_TYPE_ALBUM";
const ARTIST_PAGE: &str = "MUSIC_PAGE_TYPE_ARTIST";

pub fn songs(response: &Value, limit: usize) -> Vec<Found> {
    nav::find_all(response, "musicResponsiveListItemRenderer")
        .into_iter()
        .filter_map(song)
        .take(limit)
        .collect()
}

fn song(row: &Value) -> Option<Found> {
    let id = nav::string(row, &["playlistItemData", "videoId"])?.to_owned();
    let columns = row.get("flexColumns")?.as_array()?;
    let title = column_text(columns, 0)?;

    let byline = columns.get(1);
    let artist = byline.and_then(|column| linked_run(column, ARTIST_PAGE));
    let (album, album_id) = byline
        .and_then(|column| browse_run(column, ALBUM_PAGE))
        .map_or((None, None), |(name, id)| (Some(name), Some(id)));

    Some(Found {
        id,
        title,
        artist: artist.or_else(|| byline.and_then(plain_lead)),
        album,
        album_id,
        duration: byline.and_then(duration_run),
        cover_url: None,
        explicit: is_explicit(row),
    })
}

fn album_track(
    row: &Value,
    album: &str,
    album_id: &str,
    artist: Option<&str>,
    cover: Option<&str>,
) -> Option<Found> {
    let id = nav::string(row, &["playlistItemData", "videoId"])?.to_owned();
    let columns = row.get("flexColumns")?.as_array()?;
    let title = column_text(columns, 0)?;

    let byline = columns.get(1);

    Some(Found {
        id,
        title,
        artist: byline
            .and_then(|column| linked_run(column, ARTIST_PAGE))
            .or_else(|| artist.map(str::to_owned)),
        album: Some(album.to_owned()),
        album_id: Some(album_id.to_owned()),
        duration: fixed_duration(row).or_else(|| byline.and_then(duration_run)),
        cover_url: cover.map(str::to_owned),
        explicit: is_explicit(row),
    })
}

pub fn album(response: &Value, album_id: &str) -> Option<FoundAlbum> {
    let header = nav::find_first(response, "musicResponsiveHeaderRenderer")?;
    let title = nav::runs_text(header, &["title"])?;

    let artist = nav::runs(header, &["straplineTextOne"])
        .iter()
        .find_map(|run| run.get("text")?.as_str())
        .map(str::to_owned)
        .or_else(|| {
            nav::runs(header, &["subtitle"]).iter().find_map(|run| {
                browse_type(run)
                    .is_some_and(|t| t == ARTIST_PAGE)
                    .then(|| run.get("text")?.as_str().map(str::to_owned))?
            })
        });

    let cover = cover_at(
        header,
        &[
            "thumbnail",
            "musicThumbnailRenderer",
            "thumbnail",
            "thumbnails",
        ],
    );

    let tracks = nav::find_all(response, "musicResponsiveListItemRenderer")
        .into_iter()
        .filter_map(|row| album_track(row, &title, album_id, artist.as_deref(), cover.as_deref()))
        .collect();

    Some(FoundAlbum {
        id: album_id.to_owned(),
        title,
        artist,
        year: year_from_subtitle(header),
        cover_url: cover,
        tracks,
    })
}

pub fn radio(response: &Value, seed: &str, limit: usize) -> Vec<Found> {
    nav::find_all(response, "playlistPanelVideoRenderer")
        .into_iter()
        .filter_map(|row| radio_track(row, seed))
        .take(limit)
        .collect()
}

fn radio_track(row: &Value, seed: &str) -> Option<Found> {
    let id = nav::string(row, &["videoId"])?.to_owned();

    if id == seed {
        return None;
    }

    let byline = row.get("longBylineText");
    let (album, album_id) = byline
        .and_then(|column| browse_run(column, ALBUM_PAGE))
        .map_or((None, None), |(name, id)| (Some(name), Some(id)));

    Some(Found {
        id,
        title: nav::runs_text(row, &["title"])?,
        artist: byline
            .and_then(|column| linked_run(column, ARTIST_PAGE))
            .or_else(|| byline.and_then(plain_lead)),
        album,
        album_id,
        duration: nav::runs_text(row, &["lengthText"])
            .as_deref()
            .and_then(clock),
        cover_url: cover_at(row, &["thumbnail", "thumbnails"]),
        explicit: is_explicit(row),
    })
}

fn column_text(columns: &[Value], index: usize) -> Option<String> {
    nav::runs_text(
        columns.get(index)?,
        &["musicResponsiveListItemFlexColumnRenderer", "text"],
    )
}

fn column_runs(column: &Value) -> &[Value] {
    const CONTAINERS: [&[&str]; 3] = [
        &["musicResponsiveListItemFlexColumnRenderer", "text"],
        &["musicResponsiveListItemFixedColumnRenderer", "text"],
        &["text"],
    ];

    CONTAINERS
        .iter()
        .map(|keys| nav::runs(column, keys))
        .find(|runs| !runs.is_empty())
        .unwrap_or_else(|| {
            column
                .get("runs")
                .and_then(Value::as_array)
                .map_or(&[], Vec::as_slice)
        })
}

fn cover_at(value: &Value, keys: &[&str]) -> Option<String> {
    largest_thumbnail(nav::path(value, keys)?).map(upscale_cover)
}

fn fixed_duration(row: &Value) -> Option<u32> {
    row.get("fixedColumns")?
        .as_array()?
        .iter()
        .find_map(duration_run)
}

fn browse_type(run: &Value) -> Option<&str> {
    nav::string(
        run,
        &[
            "navigationEndpoint",
            "browseEndpoint",
            "browseEndpointContextSupportedConfigs",
            "browseEndpointContextMusicConfig",
            "pageType",
        ],
    )
}

fn linked_run(column: &Value, page_type: &str) -> Option<String> {
    column_runs(column).iter().find_map(|run| {
        (browse_type(run)? == page_type).then(|| run.get("text")?.as_str().map(str::to_owned))?
    })
}

fn browse_run(column: &Value, page_type: &str) -> Option<(String, String)> {
    column_runs(column).iter().find_map(|run| {
        if browse_type(run)? != page_type {
            return None;
        }
        let id = nav::string(run, &["navigationEndpoint", "browseEndpoint", "browseId"])?;
        Some((run.get("text")?.as_str()?.to_owned(), id.to_owned()))
    })
}

fn plain_lead(column: &Value) -> Option<String> {
    column_runs(column).iter().find_map(|run| {
        if run.get("navigationEndpoint").is_some() {
            return None;
        }
        let text = run.get("text")?.as_str()?.trim();
        (!text.is_empty() && text != "•" && clock(text).is_none()).then(|| text.to_owned())
    })
}

fn duration_run(column: &Value) -> Option<u32> {
    column_runs(column)
        .iter()
        .filter_map(|run| clock(run.get("text")?.as_str()?))
        .next_back()
}

fn clock(text: &str) -> Option<u32> {
    let text = text.trim();
    if !text.contains(':') {
        return None;
    }

    text.split(':').try_fold(0_u32, |total, part| {
        let value: u32 = part.trim().parse().ok()?;
        total.checked_mul(60)?.checked_add(value)
    })
}

fn year_from_subtitle(header: &Value) -> Option<u32> {
    nav::runs(header, &["subtitle"])
        .iter()
        .filter_map(|run| run.get("text")?.as_str()?.trim().parse::<u32>().ok())
        .find(|year| (1000..=9999).contains(year))
}

fn largest_thumbnail(thumbnails: &Value) -> Option<String> {
    let list = thumbnails.as_array()?;
    let best = list
        .iter()
        .max_by_key(|thumb| thumb.get("width").and_then(Value::as_u64).unwrap_or(0))?;
    Some(best.get("url")?.as_str()?.to_owned())
}

fn upscale_cover(url: String) -> String {
    match url.rfind("=w") {
        Some(index) => format!("{}=w544-h544-l90-rj", &url[..index]),
        None => url,
    }
}

fn is_explicit(row: &Value) -> bool {
    row.get("badges").is_some_and(|badges| {
        nav::find_all(badges, "musicInlineBadgeRenderer")
            .iter()
            .any(|badge| nav::string(badge, &["icon", "iconType"]) == Some("MUSIC_EXPLICIT_BADGE"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading fixture {path}: {e}"));
        serde_json::from_str(&text).expect("fixture is valid json")
    }

    #[test]
    fn a_search_response_yields_songs() {
        let found = songs(&fixture("search_songs.json"), 20);

        assert_eq!(found.len(), 6, "fixture holds six rows");

        let first = &found[0];
        assert_eq!(first.id, "pYHEpDnvVPk");
        assert_eq!(first.title, "Reckoner");
        assert_eq!(first.artist.as_deref(), Some("Radiohead"));
        assert_eq!(first.album.as_deref(), Some("In Rainbows"));
        assert_eq!(first.duration, Some(291));
    }

    #[test]
    fn a_song_carries_the_browse_id_of_its_album() {
        let found = songs(&fixture("search_songs.json"), 20);
        assert_eq!(found[0].album_id.as_deref(), Some("MPREb_R6C9lU4QEg2"));
    }

    #[test]
    fn every_song_has_an_id_and_a_title() {
        for song in songs(&fixture("search_songs.json"), 20) {
            assert!(!song.id.is_empty(), "{song:?} has no id");
            assert!(!song.title.is_empty(), "{song:?} has no title");
        }
    }

    #[test]
    fn a_limit_truncates_the_results() {
        assert_eq!(songs(&fixture("search_songs.json"), 2).len(), 2);
        assert_eq!(songs(&fixture("search_songs.json"), 0).len(), 0);
    }

    #[test]
    fn an_unreadable_row_is_dropped_rather_than_failing_the_response() {
        let response = json!({
            "contents": [
                { "musicResponsiveListItemRenderer": { "flexColumns": [] } },
                { "musicResponsiveListItemRenderer": {
                    "playlistItemData": { "videoId": "abc" },
                    "flexColumns": [{ "musicResponsiveListItemFlexColumnRenderer": {
                        "text": { "runs": [{ "text": "Real Song" }] } } }]
                } }
            ]
        });

        let found = songs(&response, 20);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Real Song");
    }

    #[test]
    fn a_response_with_nothing_in_it_yields_nothing() {
        assert!(songs(&json!({ "contents": [] }), 20).is_empty());
        assert!(songs(&json!({}), 20).is_empty());
    }

    #[test]
    fn a_clock_reads_as_seconds() {
        assert_eq!(clock("4:51"), Some(291));
        assert_eq!(clock("0:30"), Some(30));
        assert_eq!(clock("1:02:03"), Some(3723));
        assert_eq!(clock("  3:07 "), Some(187));
    }

    #[test]
    fn text_that_is_not_a_clock_reads_as_no_duration() {
        assert_eq!(clock("Radiohead"), None);
        assert_eq!(clock(""), None);
        assert_eq!(clock("Song"), None);
        assert_eq!(clock("4:xx"), None);
    }

    #[test]
    fn a_cover_url_is_asked_for_at_cover_resolution() {
        assert_eq!(
            upscale_cover("https://host/img=w60-h60-l90-rj".to_owned()),
            "https://host/img=w544-h544-l90-rj"
        );
    }

    #[test]
    fn a_url_with_no_size_suffix_is_left_alone() {
        let plain = "https://host/img.jpg".to_owned();
        assert_eq!(upscale_cover(plain.clone()), plain);
    }

    #[test]
    fn the_largest_thumbnail_wins_regardless_of_order() {
        let thumbs = json!([
            { "url": "small", "width": 60 },
            { "url": "big", "width": 544 },
            { "url": "middle", "width": 120 }
        ]);
        assert_eq!(largest_thumbnail(&thumbs).as_deref(), Some("big"));
    }

    #[test]
    fn a_row_without_an_album_still_reads_its_artist_and_duration() {
        let row = json!({
            "playlistItemData": { "videoId": "xyz" },
            "flexColumns": [
                { "musicResponsiveListItemFlexColumnRenderer": {
                    "text": { "runs": [{ "text": "Lone Single" }] } } },
                { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [
                    { "text": "Some Artist", "navigationEndpoint": { "browseEndpoint": {
                        "browseId": "UC123",
                        "browseEndpointContextSupportedConfigs": {
                            "browseEndpointContextMusicConfig": { "pageType": ARTIST_PAGE } } } } },
                    { "text": " • " },
                    { "text": "3:07" }
                ] } } }
            ]
        });

        let parsed = song(&row).expect("row parses");
        assert_eq!(parsed.artist.as_deref(), Some("Some Artist"));
        assert_eq!(parsed.album, None);
        assert_eq!(parsed.album_id, None);
        assert_eq!(parsed.duration, Some(187));
    }

    #[test]
    fn an_album_track_reads_its_duration_from_the_fixed_column() {
        let row = json!({
            "playlistItemData": { "videoId": "t1" },
            "flexColumns": [
                { "musicResponsiveListItemFlexColumnRenderer": {
                    "text": { "runs": [{ "text": "15 Step" }] } } },
                { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [] } } }
            ],
            "fixedColumns": [
                { "musicResponsiveListItemFixedColumnRenderer": {
                    "text": { "runs": [{ "text": "3:58" }] } } }
            ]
        });

        let parsed =
            album_track(&row, "In Rainbows", "MPRE1", Some("Radiohead"), None).expect("row parses");
        assert_eq!(parsed.duration, Some(238));
        assert_eq!(parsed.artist.as_deref(), Some("Radiohead"));
    }

    #[test]
    fn a_radio_row_reads_the_artist_off_its_byline() {
        let row = json!({
            "videoId": "r1",
            "title": { "runs": [{ "text": "Jigsaw Falling Into Place" }] },
            "lengthText": { "runs": [{ "text": "4:09" }] },
            "longBylineText": { "runs": [
                { "text": "Radiohead", "navigationEndpoint": { "browseEndpoint": {
                    "browseId": "UC1",
                    "browseEndpointContextSupportedConfigs": {
                        "browseEndpointContextMusicConfig": { "pageType": ARTIST_PAGE } } } } },
                { "text": " • " },
                { "text": "In Rainbows", "navigationEndpoint": { "browseEndpoint": {
                    "browseId": "MPRE1",
                    "browseEndpointContextSupportedConfigs": {
                        "browseEndpointContextMusicConfig": { "pageType": ALBUM_PAGE } } } } },
                { "text": " • " },
                { "text": "2007" }
            ] }
        });

        let parsed = radio_track(&row, "seed").expect("row parses");
        assert_eq!(parsed.artist.as_deref(), Some("Radiohead"));
        assert_eq!(parsed.album.as_deref(), Some("In Rainbows"));
        assert_eq!(parsed.album_id.as_deref(), Some("MPRE1"));
        assert_eq!(parsed.duration, Some(249));
    }

    #[test]
    fn a_station_does_not_return_its_own_seed() {
        let row = json!({
            "videoId": "seed",
            "title": { "runs": [{ "text": "The Seed" }] }
        });

        assert!(radio_track(&row, "seed").is_none());
        assert!(radio_track(&row, "other").is_some());
    }

    #[test]
    fn an_album_takes_its_cover_and_not_the_artist_photo() {
        let header = json!({
            "musicResponsiveHeaderRenderer": {
                "title": { "runs": [{ "text": "In Rainbows" }] },
                "subtitle": { "runs": [{ "text": "Album" }, { "text": "2007" }] },
                "straplineTextOne": { "runs": [{ "text": "Radiohead" }] },
                "thumbnail": { "musicThumbnailRenderer": { "thumbnail": { "thumbnails": [
                    { "url": "https://host/sleeve=w544-h544-l90-rj", "width": 544 }
                ] } } },
                "straplineThumbnail": { "musicThumbnailRenderer": { "thumbnail": { "thumbnails": [
                    { "url": "https://host/bandphoto=w544-h544-l90-rj", "width": 544 }
                ] } } }
            }
        });

        let parsed = album(&header, "MPRE1").expect("header parses");
        let cover = parsed.cover_url.expect("a cover");

        assert!(cover.contains("sleeve"), "took {cover}");
        assert!(
            !cover.contains("bandphoto"),
            "took the artist photo: {cover}"
        );
    }

    #[test]
    fn an_explicit_badge_is_noticed() {
        let row = json!({
            "playlistItemData": { "videoId": "e1" },
            "flexColumns": [{ "musicResponsiveListItemFlexColumnRenderer": {
                "text": { "runs": [{ "text": "Rude Song" }] } } }],
            "badges": [{ "musicInlineBadgeRenderer": {
                "icon": { "iconType": "MUSIC_EXPLICIT_BADGE" } } }]
        });

        assert!(song(&row).expect("row parses").explicit);
    }

    #[test]
    fn a_row_with_no_badge_is_not_explicit() {
        let found = songs(&fixture("search_songs.json"), 20);
        assert!(found.iter().any(|s| !s.explicit));
    }
}
