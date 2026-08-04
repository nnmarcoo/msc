//! Deciding whether a search result is already in the library.
//!
//! A recording's id cannot answer this. YouTube gives the same recording
//! different ids depending on how it was reached — searching "radiohead
//! reckoner" yields `pYHEpDnvVPk`, while the same track in the In Rainbows
//! listing is `_uofQD-N6UI` — so an id that is not in the library proves
//! nothing, and matching on it reports a track the user already owns as
//! missing, then downloads a second copy of it.
//!
//! So the match is on what the music *is*: title and artist, folded.
//!
//! Duration deliberately does *not* gate the match. It was tried as a tolerance
//! and it refused real matches: YouTube states J'OUVERT at 235s against the
//! album's own 252s, and SAN MARCOS at 287s against 309s — differences of
//! seventeen and twenty-two seconds for the same recording, because the store's
//! copy is edited differently. Even one library held NEW ORLEANS twice with an
//! 8.5s spread. Any tolerance loose enough to admit those is loose enough to be
//! meaningless, so length is used only to *choose between* several tracks that
//! already agree on title and artist, never to reject the only one. Being wrong
//! about which of two near-identical copies is meant costs nothing; refusing a
//! track the user owns sends them to download a second copy.
//!
//! Folding is deliberately more aggressive than [`crate::Track`]'s own
//! comparisons. A store's title carries decoration a tagged file usually does
//! not: "Creep (Remastered 2016)", "Nude - 2007 Remaster", "Weird Fishes /
//! Arpeggi (feat. Someone)". [`fold`] removes bracketed and dash-suffixed
//! qualifiers so those meet the plain title already on disk, which is the whole
//! point — the alternative is a library full of near-duplicates.
//!
//! It does *not* remove a qualifier that names a different recording. "Creep
//! (Acoustic)" and "Creep (Live)" are not the studio take, and treating them as
//! already-owned would silently refuse a download the user meant. [`VARIANTS`]
//! is that list, and it is the reason this cannot be a blanket "drop everything
//! in brackets".

use crate::{Library, Track};

const REMASTER_HINTS: [&str; 8] = [
    "remaster",
    "remastered",
    "mono",
    "stereo",
    "deluxe",
    "bonus track",
    "album version",
    "original mix",
];

const VARIANTS: [&str; 10] = [
    "live",
    "acoustic",
    "instrumental",
    "demo",
    "remix",
    "edit",
    "cover",
    "karaoke",
    "sped up",
    "slowed",
];

pub fn already_held(
    library: &Library,
    title: &str,
    artist: Option<&str>,
    duration: Option<u32>,
) -> Option<i64> {
    let wanted_title = fold(title);
    if wanted_title.is_empty() {
        return None;
    }

    let wanted_artist = artist.map(fold_plain);

    let mut candidates = library
        .available()
        .filter(|track| matches(track, &wanted_title, wanted_artist.as_deref()))
        .peekable();

    candidates.peek()?;

    let closest = match duration {
        Some(wanted) => candidates.min_by(|a, b| {
            gap(a, wanted)
                .partial_cmp(&gap(b, wanted))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        None => candidates.next(),
    };

    closest.and_then(Track::id)
}

fn gap(track: &Track, wanted: u32) -> f32 {
    (track.duration() - wanted as f32).abs()
}

fn matches(track: &Track, title: &str, artist: Option<&str>) -> bool {
    let Some(held_title) = track.title() else {
        return false;
    };

    if fold(held_title) != title {
        return false;
    }

    let Some(artist) = artist else {
        return true;
    };

    track
        .album_artist()
        .or_else(|| track.track_artist())
        .map(fold_plain)
        .is_some_and(|held| credits_agree(&held, artist))
}

fn credits_agree(held: &str, wanted: &str) -> bool {
    held == wanted || held.starts_with(wanted) || wanted.starts_with(held)
}

pub fn fold(title: &str) -> String {
    let lowered = title.to_lowercase();
    let mut kept = String::with_capacity(lowered.len());
    let mut depth = 0_usize;

    for character in lowered.chars() {
        match character {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => kept.push(character),
            _ => {}
        }
    }

    let qualifiers: Vec<&str> = bracketed(&lowered);

    if qualifiers.iter().any(|qualifier| is_variant(qualifier)) {
        return fold_plain(&lowered);
    }

    if let Some((before, after)) = kept.rsplit_once(" - ") {
        let after = after.trim();
        if is_variant(after) {
            return fold_plain(&kept);
        }
        if !after.is_empty() && (is_decoration(after) || after.chars().all(|c| !c.is_alphabetic()))
        {
            kept = before.to_owned();
        }
    }

    fold_plain(&kept)
}

fn bracketed(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = text;

    while let Some(open) = rest.find(['(', '[']) {
        let after = &rest[open + 1..];
        match after.find([')', ']']) {
            Some(close) => {
                found.push(after[..close].trim());
                rest = &after[close + 1..];
            }
            None => break,
        }
    }

    found
}

fn is_variant(qualifier: &str) -> bool {
    VARIANTS
        .iter()
        .any(|variant| qualifier == *variant || qualifier.starts_with(&format!("{variant} ")))
}

fn is_decoration(qualifier: &str) -> bool {
    REMASTER_HINTS.iter().any(|hint| qualifier.contains(hint))
}

const JOINERS: [char; 4] = ['\'', '\u{2019}', '.', '\u{00b7}'];

fn fold_plain(text: &str) -> String {
    let mut folded = String::with_capacity(text.len());
    let mut spaced = false;

    for character in text.to_lowercase().chars() {
        if character.is_alphanumeric() {
            folded.push(character);
            spaced = false;
        } else if !JOINERS.contains(&character) && !folded.is_empty() && !spaced {
            folded.push(' ');
            spaced = true;
        }
    }

    folded.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_title_folds_to_itself() {
        assert_eq!(fold("Reckoner"), "reckoner");
        assert_eq!(fold("15 Step"), "15 step");
    }

    #[test]
    fn case_and_punctuation_stop_mattering() {
        assert_eq!(fold("Weird Fishes / Arpeggi"), fold("weird fishes arpeggi"));
    }

    /// An apostrophe joins a word rather than breaking it, and a store and a
    /// tagged file rarely agree on which one to use.
    #[test]
    fn an_apostrophe_does_not_split_a_word() {
        assert_eq!(fold("Don't Panic"), fold("Dont Panic"));
        assert_eq!(fold("Don\u{2019}t Panic"), fold("Don't Panic"));
        assert_eq!(fold("Everything's Not Lost"), fold("Everythings Not Lost"));
    }

    /// The case this exists for: a store decorates a title that a tagged file
    /// states plainly, and they must still meet.
    #[test]
    fn a_remaster_note_does_not_make_a_different_song() {
        assert_eq!(fold("Creep (Remastered 2016)"), fold("Creep"));
        assert_eq!(fold("Nude - 2007 Remaster"), fold("Nude"));
        assert_eq!(fold("Karma Police (Deluxe)"), fold("Karma Police"));
        assert_eq!(fold("No Surprises [Album Version]"), fold("No Surprises"));
    }

    /// The case that must not be folded away: these are other recordings, and
    /// refusing to download one because the studio take is held would be wrong.
    #[test]
    fn a_variant_stays_a_different_recording() {
        assert_ne!(fold("Creep (Acoustic)"), fold("Creep"));
        assert_ne!(fold("Creep (Live)"), fold("Creep"));
        assert_ne!(fold("Creep (Live at Glastonbury)"), fold("Creep"));
        assert_ne!(fold("Idioteque - Remix"), fold("Idioteque"));
        assert_ne!(fold("Reckoner (Sped Up)"), fold("Reckoner"));
    }

    #[test]
    fn a_featured_credit_folds_into_the_title() {
        assert_eq!(fold("Song (feat. Someone)"), fold("Song"));
    }

    #[test]
    fn an_empty_or_symbol_only_title_folds_to_nothing() {
        assert_eq!(fold(""), "");
        assert_eq!(fold("()"), "");
    }

    #[test]
    fn a_credit_matches_when_one_side_names_a_guest() {
        assert!(credits_agree("radiohead", "radiohead"));
        assert!(credits_agree("radiohead feat someone", "radiohead"));
        assert!(credits_agree("radiohead", "radiohead feat someone"));
        assert!(!credits_agree("radiohead", "coldplay"));
    }

    #[test]
    fn different_artists_do_not_agree() {
        assert!(!credits_agree("halocene", "radiohead"));
        assert!(!credits_agree("kelly clarkson", "radiohead"));
    }
}
