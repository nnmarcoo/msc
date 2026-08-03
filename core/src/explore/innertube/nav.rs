//! Typed reads over an Innertube response.
//!
//! Innertube answers are deep, mostly irrelevant, and reshaped by YouTube
//! without notice. Modelling the whole schema in serde would mean a struct that
//! fails to deserialize the moment a field moves, taking a whole search with it.
//! These helpers instead read the few fields a result actually needs and answer
//! `None` for anything absent or reshaped, so a parser can drop one unreadable
//! row and keep the rest.
//!
//! [`find_all`] exists because the renderer a row lives in is nested at a depth
//! that differs between endpoints, and between two responses from the same
//! endpoint: search wraps rows in tabs and shelves, an album in a two-column
//! layout, radio in a playlist panel. Searching by renderer name rather than by
//! a fixed path is what lets one parser read a row wherever it was placed, and
//! is why a shelf being wrapped in a new container does not break anything.
//!
//! A match is not descended into. Row renderers do not nest — verified across
//! live search, album and radio responses — and the subtree under one is its
//! own fields, which are read by path rather than by search. Descending anyway
//! walked 7354 nodes of a search response where 203 reach every row, and paid
//! that on every keystroke.
//!
//! [`runs_text`] joins a `runs` array, which is how Innertube spells every piece
//! of display text: a string split at each point its formatting or link
//! changes. "Radiohead • In Rainbows • 4:51" arrives as five runs, and only the
//! joined string is meaningful.

use serde_json::Value;

pub fn path<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().try_fold(value, |current, key| current.get(key))
}

pub fn string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    path(value, keys)?.as_str()
}

pub fn find_all<'a>(value: &'a Value, renderer: &str) -> Vec<&'a Value> {
    let mut found = Vec::new();
    collect(value, renderer, &mut found);
    found
}

pub fn find_first<'a>(value: &'a Value, renderer: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) => map.iter().find_map(|(key, child)| {
            if key == renderer {
                Some(child)
            } else {
                find_first(child, renderer)
            }
        }),
        Value::Array(items) => items.iter().find_map(|item| find_first(item, renderer)),
        _ => None,
    }
}

fn collect<'a>(value: &'a Value, renderer: &str, found: &mut Vec<&'a Value>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == renderer {
                    found.push(child);
                } else {
                    collect(child, renderer, found);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect(item, renderer, found);
            }
        }
        _ => {}
    }
}

pub fn runs_text(value: &Value, keys: &[&str]) -> Option<String> {
    let runs = path(value, keys)?.get("runs")?.as_array()?;

    let text: String = runs
        .iter()
        .filter_map(|run| run.get("text")?.as_str())
        .collect();

    (!text.is_empty()).then_some(text)
}

pub fn runs<'a>(value: &'a Value, keys: &[&str]) -> &'a [Value] {
    path(value, keys)
        .and_then(|text| text.get("runs"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_missing_key_reads_as_absent_rather_than_faulting() {
        let value = json!({ "a": { "b": 1 } });

        assert!(path(&value, &["a", "b"]).is_some());
        assert!(path(&value, &["a", "nope"]).is_none());
        assert!(path(&value, &["nope", "b"]).is_none());
    }

    #[test]
    fn runs_join_into_one_string() {
        let value = json!({
            "title": { "runs": [{ "text": "In " }, { "text": "Rainbows" }] }
        });

        assert_eq!(
            runs_text(&value, &["title"]).as_deref(),
            Some("In Rainbows")
        );
    }

    #[test]
    fn empty_runs_read_as_absent() {
        let value = json!({ "title": { "runs": [] } });
        assert!(runs_text(&value, &["title"]).is_none());

        let blank = json!({ "title": { "runs": [{ "text": "" }] } });
        assert!(runs_text(&blank, &["title"]).is_none());
    }

    #[test]
    fn a_renderer_is_found_at_any_depth() {
        let value = json!({
            "contents": [
                { "wrapper": { "musicRow": { "id": 1 } } },
                { "deeper": { "nested": [{ "musicRow": { "id": 2 } }] } }
            ]
        });

        let rows = find_all(&value, "musicRow");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["id"], 1);
        assert_eq!(rows[1]["id"], 2);
    }

    #[test]
    fn a_match_is_not_descended_into() {
        let value = json!({
            "musicRow": { "id": 1, "inner": { "musicRow": { "id": 2 } } }
        });

        let rows = find_all(&value, "musicRow");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["id"], 1);
    }

    #[test]
    fn nothing_matching_is_an_empty_list_rather_than_an_error() {
        let value = json!({ "contents": [] });
        assert!(find_all(&value, "musicRow").is_empty());
        assert!(find_first(&value, "musicRow").is_none());
    }
}
