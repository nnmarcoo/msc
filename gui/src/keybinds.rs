//! Rebindable keyboard shortcuts: what the app can do, and which key does it.
//!
//! An [`Action`] names an intent rather than a key, so the rest of the app asks
//! "was this Play/Pause?" and never "was this Space?". That indirection is the
//! whole feature: the key becomes data the user owns, and adding a shortcut is
//! adding a variant plus a default rather than editing a match on characters.
//!
//! Matching is on `Physical::Code`, the position of the key on the keyboard,
//! not on the character it produces. A logical character is the wrong thing to
//! bind: it changes with the layout and with modifiers, so a binding captured on
//! one layout would land on a different key on another, and a shifted binding
//! would arrive as a different character than the one recorded. The position is
//! stable, which is what makes a captured binding mean the same thing later.
//!
//! Modifiers must match exactly rather than merely being present. A binding on
//! plain `S` therefore does not fire on `Ctrl+S`, which leaves `Ctrl+S` free to
//! be bound to something else — the alternative silently makes every unmodified
//! binding shadow every modified one built on the same key.
//!
//! [`Keymap::set`] enforces the one rule that keeps a keymap usable: a binding
//! belongs to at most one action. Assigning a key already in use unbinds it from
//! wherever it was and returns those actions, so the UI can say what it took
//! rather than leaving two actions racing for one key. Actions may hold no
//! binding at all, which is what makes "unbind" expressible.
//!
//! `SelectLayout` is parameterised because the ten layout slots are the same
//! action ten times over, and writing them as ten variants would mean ten arms
//! everywhere. They default to `1`-`9` then `0`, matching the number row, but
//! nothing depends on that: each is rebindable to any key like everything else.
//!
//! The on-disk form is a table of action name to binding string rather than a
//! serialization of this type, so a config stays readable and hand-editable, and
//! an unknown or malformed entry can be dropped without failing the whole file.
//! An absent entry means "use the default", while an empty string means the user
//! deliberately unbound it — a distinction a plain `Option` on disk would lose.
//!
//! `CODE_NAMES` is therefore both the spelling table and the set of keys that
//! may be bound at all, and `is_bindable` is the guard that keeps those two
//! meanings from parting: a key with no name renders as "Unknown", which is not
//! a name `parse` accepts, so binding one would appear to work and then vanish
//! on the next launch. Capture refuses such a key instead of accepting a binding
//! it cannot keep. Widening what can be bound means adding to the table, not
//! relaxing the guard.

use std::collections::HashMap;

use iced::keyboard::{
    self,
    key::{self, Physical},
};
use serde::{Deserialize, Serialize};

pub const LAYOUT_SLOTS: u8 = crate::layout::PRESET_SLOTS as u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    PlayPause,
    Next,
    Previous,
    ToggleMute,
    VolumeUp,
    VolumeDown,
    CycleLoop,
    Shuffle,
    ToggleEditMode,
    TogglePreferences,
    SelectLayout(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCategory {
    Playback,
    Interface,
    Layouts,
}

impl KeyCategory {
    pub fn all() -> &'static [Self] {
        &[Self::Playback, Self::Interface, Self::Layouts]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Playback => "Playback",
            Self::Interface => "Interface",
            Self::Layouts => "Layouts",
        }
    }
}

impl Action {
    pub fn label(self) -> String {
        match self {
            Self::PlayPause => "Play / pause".into(),
            Self::Next => "Next track".into(),
            Self::Previous => "Previous track".into(),
            Self::ToggleMute => "Mute".into(),
            Self::VolumeUp => "Volume up".into(),
            Self::VolumeDown => "Volume down".into(),
            Self::CycleLoop => "Cycle loop mode".into(),
            Self::Shuffle => "Shuffle queue".into(),
            Self::ToggleEditMode => "Edit mode".into(),
            Self::TogglePreferences => "Preferences".into(),
            Self::SelectLayout(slot) => format!("Switch to layout {slot}"),
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::PlayPause => "Start or pause the current track",
            Self::Next => "Skip to the next track in the queue",
            Self::Previous => "Go back to the previous track",
            Self::ToggleMute => "Silence playback without losing the volume level",
            Self::VolumeUp => "Raise the volume a step, unmuting if muted",
            Self::VolumeDown => "Lower the volume a step, unmuting if muted",
            Self::CycleLoop => "Cycle through off, repeat queue, and repeat track",
            Self::Shuffle => "Shuffle the tracks waiting in the queue",
            Self::ToggleEditMode => "Show the pane handles, split buttons, and locks",
            Self::TogglePreferences => "Open or close this preferences view",
            Self::SelectLayout(_) => "Switch to a saved layout preset",
        }
    }

    pub fn category(self) -> KeyCategory {
        match self {
            Self::PlayPause
            | Self::Next
            | Self::Previous
            | Self::ToggleMute
            | Self::VolumeUp
            | Self::VolumeDown
            | Self::CycleLoop
            | Self::Shuffle => KeyCategory::Playback,
            Self::ToggleEditMode | Self::TogglePreferences => KeyCategory::Interface,
            Self::SelectLayout(_) => KeyCategory::Layouts,
        }
    }

    pub fn all() -> Vec<Self> {
        let mut actions = vec![
            Self::PlayPause,
            Self::Next,
            Self::Previous,
            Self::ToggleMute,
            Self::VolumeUp,
            Self::VolumeDown,
            Self::CycleLoop,
            Self::Shuffle,
            Self::ToggleEditMode,
            Self::TogglePreferences,
        ];
        actions.extend((1..=LAYOUT_SLOTS).map(Self::SelectLayout));
        actions
    }

    fn key(self) -> String {
        match self {
            Self::PlayPause => "play_pause".into(),
            Self::Next => "next".into(),
            Self::Previous => "previous".into(),
            Self::ToggleMute => "toggle_mute".into(),
            Self::VolumeUp => "volume_up".into(),
            Self::VolumeDown => "volume_down".into(),
            Self::CycleLoop => "cycle_loop".into(),
            Self::Shuffle => "shuffle".into(),
            Self::ToggleEditMode => "toggle_edit_mode".into(),
            Self::TogglePreferences => "toggle_preferences".into(),
            Self::SelectLayout(slot) => format!("select_layout_{slot}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub code: key::Code,
}

impl KeyBinding {
    pub fn plain(code: key::Code) -> Self {
        Self {
            ctrl: false,
            shift: false,
            alt: false,
            code,
        }
    }

    pub fn matches(self, physical: Physical, modifiers: keyboard::Modifiers) -> bool {
        let Physical::Code(code) = physical else {
            return false;
        };
        code == self.code
            && modifiers.control() == self.ctrl
            && modifiers.shift() == self.shift
            && modifiers.alt() == self.alt
    }

    fn render(self, name: fn(key::Code) -> &'static str) -> String {
        let mut text = String::new();
        if self.ctrl {
            text.push_str("Ctrl+");
        }
        if self.shift {
            text.push_str("Shift+");
        }
        if self.alt {
            text.push_str("Alt+");
        }
        text.push_str(name(self.code));
        text
    }

    pub fn stored(self) -> String {
        self.render(code_name)
    }

    pub fn shown(self) -> String {
        self.render(code_label)
    }

    pub fn parse(text: &str) -> Option<Self> {
        let mut binding = Self::plain(key::Code::Space);
        let mut parts = text.split('+').peekable();

        loop {
            match parts.peek() {
                Some(&"Ctrl") => binding.ctrl = true,
                Some(&"Shift") => binding.shift = true,
                Some(&"Alt") => binding.alt = true,
                _ => break,
            }
            parts.next();
        }

        binding.code = name_to_code(parts.next()?)?;
        parts.next().is_none().then_some(binding)
    }
}

const CODE_NAMES: &[(key::Code, &str)] = &[
    (key::Code::ArrowRight, "ArrowRight"),
    (key::Code::ArrowLeft, "ArrowLeft"),
    (key::Code::ArrowUp, "ArrowUp"),
    (key::Code::ArrowDown, "ArrowDown"),
    (key::Code::Equal, "Equal"),
    (key::Code::Minus, "Minus"),
    (key::Code::Digit0, "0"),
    (key::Code::Digit1, "1"),
    (key::Code::Digit2, "2"),
    (key::Code::Digit3, "3"),
    (key::Code::Digit4, "4"),
    (key::Code::Digit5, "5"),
    (key::Code::Digit6, "6"),
    (key::Code::Digit7, "7"),
    (key::Code::Digit8, "8"),
    (key::Code::Digit9, "9"),
    (key::Code::KeyA, "A"),
    (key::Code::KeyB, "B"),
    (key::Code::KeyC, "C"),
    (key::Code::KeyD, "D"),
    (key::Code::KeyE, "E"),
    (key::Code::KeyF, "F"),
    (key::Code::KeyG, "G"),
    (key::Code::KeyH, "H"),
    (key::Code::KeyI, "I"),
    (key::Code::KeyJ, "J"),
    (key::Code::KeyK, "K"),
    (key::Code::KeyL, "L"),
    (key::Code::KeyM, "M"),
    (key::Code::KeyN, "N"),
    (key::Code::KeyO, "O"),
    (key::Code::KeyP, "P"),
    (key::Code::KeyQ, "Q"),
    (key::Code::KeyR, "R"),
    (key::Code::KeyS, "S"),
    (key::Code::KeyT, "T"),
    (key::Code::KeyU, "U"),
    (key::Code::KeyV, "V"),
    (key::Code::KeyW, "W"),
    (key::Code::KeyX, "X"),
    (key::Code::KeyY, "Y"),
    (key::Code::KeyZ, "Z"),
    (key::Code::Space, "Space"),
    (key::Code::Enter, "Enter"),
    (key::Code::Escape, "Escape"),
    (key::Code::Backspace, "Backspace"),
    (key::Code::Tab, "Tab"),
    (key::Code::Delete, "Delete"),
    (key::Code::Home, "Home"),
    (key::Code::End, "End"),
    (key::Code::PageUp, "PageUp"),
    (key::Code::PageDown, "PageDown"),
    (key::Code::F1, "F1"),
    (key::Code::F2, "F2"),
    (key::Code::F3, "F3"),
    (key::Code::F4, "F4"),
    (key::Code::F5, "F5"),
    (key::Code::F6, "F6"),
    (key::Code::F7, "F7"),
    (key::Code::F8, "F8"),
    (key::Code::F9, "F9"),
    (key::Code::F10, "F10"),
    (key::Code::F11, "F11"),
    (key::Code::F12, "F12"),
    (key::Code::BracketLeft, "BracketLeft"),
    (key::Code::BracketRight, "BracketRight"),
    (key::Code::Backslash, "Backslash"),
    (key::Code::Semicolon, "Semicolon"),
    (key::Code::Quote, "Quote"),
    (key::Code::Comma, "Comma"),
    (key::Code::Period, "Period"),
    (key::Code::Slash, "Slash"),
    (key::Code::Backquote, "Backquote"),
];

fn code_name(code: key::Code) -> &'static str {
    CODE_NAMES
        .iter()
        .find(|(candidate, _)| *candidate == code)
        .map_or("Unknown", |(_, name)| name)
}

fn name_to_code(name: &str) -> Option<key::Code> {
    CODE_NAMES
        .iter()
        .find(|(_, candidate)| *candidate == name)
        .map(|(code, _)| *code)
}

fn code_label(code: key::Code) -> &'static str {
    match code {
        key::Code::ArrowRight => "Right",
        key::Code::ArrowLeft => "Left",
        key::Code::ArrowUp => "Up",
        key::Code::ArrowDown => "Down",
        key::Code::Equal => "=",
        key::Code::Minus => "-",
        key::Code::BracketLeft => "[",
        key::Code::BracketRight => "]",
        key::Code::Backslash => "\\",
        key::Code::Semicolon => ";",
        key::Code::Quote => "'",
        key::Code::Comma => ",",
        key::Code::Period => ".",
        key::Code::Slash => "/",
        key::Code::Backquote => "`",
        other => code_name(other),
    }
}

pub fn is_bindable(code: key::Code) -> bool {
    CODE_NAMES.iter().any(|(candidate, _)| *candidate == code)
}

pub fn is_modifier(code: key::Code) -> bool {
    matches!(
        code,
        key::Code::ControlLeft
            | key::Code::ControlRight
            | key::Code::ShiftLeft
            | key::Code::ShiftRight
            | key::Code::AltLeft
            | key::Code::AltRight
            | key::Code::SuperLeft
            | key::Code::SuperRight
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    bindings: HashMap<Action, KeyBinding>,
}

const LAYOUT_DIGITS: [key::Code; LAYOUT_SLOTS as usize] = [
    key::Code::Digit1,
    key::Code::Digit2,
    key::Code::Digit3,
    key::Code::Digit4,
    key::Code::Digit5,
    key::Code::Digit6,
    key::Code::Digit7,
    key::Code::Digit8,
    key::Code::Digit9,
    key::Code::Digit0,
];

impl Default for Keymap {
    fn default() -> Self {
        let plain = KeyBinding::plain;
        let mut bindings = HashMap::new();

        bindings.insert(Action::PlayPause, plain(key::Code::Space));
        bindings.insert(Action::Next, plain(key::Code::ArrowRight));
        bindings.insert(Action::Previous, plain(key::Code::ArrowLeft));
        bindings.insert(Action::ToggleMute, plain(key::Code::KeyM));
        bindings.insert(Action::VolumeUp, plain(key::Code::ArrowUp));
        bindings.insert(Action::VolumeDown, plain(key::Code::ArrowDown));
        bindings.insert(Action::CycleLoop, plain(key::Code::KeyR));
        bindings.insert(Action::Shuffle, plain(key::Code::KeyH));
        bindings.insert(Action::ToggleEditMode, plain(key::Code::KeyE));
        bindings.insert(Action::TogglePreferences, plain(key::Code::KeyS));

        for (index, code) in LAYOUT_DIGITS.into_iter().enumerate() {
            let slot = u8::try_from(index).unwrap_or(0) + 1;
            bindings.insert(Action::SelectLayout(slot), plain(code));
        }

        Self { bindings }
    }
}

impl Keymap {
    pub fn resolve(&self, physical: Physical, modifiers: keyboard::Modifiers) -> Option<Action> {
        self.bindings
            .iter()
            .find(|(_, binding)| binding.matches(physical, modifiers))
            .map(|(action, _)| *action)
    }

    pub fn binding(&self, action: Action) -> Option<KeyBinding> {
        self.bindings.get(&action).copied()
    }

    pub fn set(&mut self, action: Action, binding: KeyBinding) -> Vec<Action> {
        let mut displaced = Vec::new();
        self.bindings.retain(|held, existing| {
            let clash = *existing == binding && *held != action;
            if clash {
                displaced.push(*held);
            }
            !clash
        });
        self.bindings.insert(action, binding);
        displaced
    }

    pub fn clear(&mut self, action: Action) {
        self.bindings.remove(&action);
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KeymapFile(HashMap<String, String>);

impl From<&Keymap> for KeymapFile {
    fn from(keymap: &Keymap) -> Self {
        Self(
            Action::all()
                .into_iter()
                .map(|action| {
                    let binding = keymap.binding(action).map(KeyBinding::stored);
                    (action.key(), binding.unwrap_or_default())
                })
                .collect(),
        )
    }
}

impl From<KeymapFile> for Keymap {
    fn from(file: KeymapFile) -> Self {
        let defaults = Self::default();
        let bindings = Action::all()
            .into_iter()
            .filter_map(|action| {
                let binding = match file.0.get(&action.key()) {
                    None => defaults.binding(action),
                    Some(text) if text.is_empty() => None,
                    Some(text) => KeyBinding::parse(text),
                };
                Some((action, binding?))
            })
            .collect();

        Self { bindings }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modifiers(ctrl: bool, shift: bool) -> keyboard::Modifiers {
        let mut held = keyboard::Modifiers::empty();
        if ctrl {
            held |= keyboard::Modifiers::CTRL;
        }
        if shift {
            held |= keyboard::Modifiers::SHIFT;
        }
        held
    }

    fn pressed(code: key::Code) -> Physical {
        Physical::Code(code)
    }

    #[test]
    fn the_defaults_bind_every_action() {
        let keymap = Keymap::default();
        for action in Action::all() {
            assert!(
                keymap.binding(action).is_some(),
                "{} has no default key",
                action.label()
            );
        }
    }

    #[test]
    fn no_two_default_actions_share_a_key() {
        let keymap = Keymap::default();
        let mut seen: Vec<KeyBinding> = Vec::new();

        for action in Action::all() {
            let binding = keymap.binding(action).expect("a default");
            assert!(
                !seen.contains(&binding),
                "{} reuses {}",
                action.label(),
                binding.shown()
            );
            seen.push(binding);
        }
    }

    #[test]
    fn every_layout_slot_has_a_number_key() {
        let keymap = Keymap::default();
        for slot in 1..=LAYOUT_SLOTS {
            assert!(keymap.binding(Action::SelectLayout(slot)).is_some());
        }
    }

    #[test]
    fn a_press_resolves_to_the_action_holding_it() {
        let keymap = Keymap::default();
        assert_eq!(
            keymap.resolve(pressed(key::Code::Space), keyboard::Modifiers::empty()),
            Some(Action::PlayPause)
        );
    }

    #[test]
    fn the_arrow_keys_are_the_transport_and_the_volume() {
        let keymap = Keymap::default();
        let empty = keyboard::Modifiers::empty();

        for (code, action) in [
            (key::Code::ArrowUp, Action::VolumeUp),
            (key::Code::ArrowDown, Action::VolumeDown),
            (key::Code::ArrowRight, Action::Next),
            (key::Code::ArrowLeft, Action::Previous),
        ] {
            assert_eq!(keymap.resolve(pressed(code), empty), Some(action));
        }
    }

    #[test]
    fn a_modified_press_does_not_fire_a_plain_binding() {
        let keymap = Keymap::default();
        assert_eq!(
            keymap.resolve(pressed(key::Code::KeyS), modifiers(true, false)),
            None,
            "Ctrl+S fired the binding held by plain S"
        );
    }

    #[test]
    fn rebinding_takes_the_key_from_whoever_held_it() {
        let mut keymap = Keymap::default();
        let space = KeyBinding::plain(key::Code::Space);

        let displaced = keymap.set(Action::Shuffle, space);

        assert_eq!(displaced, vec![Action::PlayPause]);
        assert_eq!(keymap.binding(Action::PlayPause), None);
        assert_eq!(
            keymap.resolve(pressed(key::Code::Space), keyboard::Modifiers::empty()),
            Some(Action::Shuffle)
        );
    }

    #[test]
    fn rebinding_an_action_to_its_own_key_displaces_nothing() {
        let mut keymap = Keymap::default();
        let space = KeyBinding::plain(key::Code::Space);

        assert!(keymap.set(Action::PlayPause, space).is_empty());
        assert_eq!(keymap.binding(Action::PlayPause), Some(space));
    }

    #[test]
    fn a_cleared_action_answers_no_key() {
        let mut keymap = Keymap::default();
        keymap.clear(Action::PlayPause);

        assert_eq!(keymap.binding(Action::PlayPause), None);
        assert_eq!(
            keymap.resolve(pressed(key::Code::Space), keyboard::Modifiers::empty()),
            None
        );
    }

    #[test]
    fn a_binding_survives_a_round_trip_through_the_file() {
        let mut keymap = Keymap::default();
        keymap.set(
            Action::Shuffle,
            KeyBinding {
                ctrl: true,
                shift: true,
                alt: false,
                code: key::Code::KeyJ,
            },
        );

        let restored = Keymap::from(KeymapFile::from(&keymap));

        assert_eq!(restored, keymap);
    }

    #[test]
    fn an_unbound_action_stays_unbound_across_a_round_trip() {
        let mut keymap = Keymap::default();
        keymap.clear(Action::PlayPause);

        let restored = Keymap::from(KeymapFile::from(&keymap));

        assert_eq!(
            restored.binding(Action::PlayPause),
            None,
            "an unbound action came back with its default"
        );
    }

    #[test]
    fn an_absent_entry_falls_back_to_the_default() {
        let restored = Keymap::from(KeymapFile::default());
        assert_eq!(restored, Keymap::default());
    }

    #[test]
    fn an_unreadable_binding_is_dropped_rather_than_guessed() {
        let mut file = KeymapFile::from(&Keymap::default());
        file.0
            .insert(Action::PlayPause.key(), "Ctrl+Nonsense".into());

        let restored = Keymap::from(file);

        assert_eq!(restored.binding(Action::PlayPause), None);
        assert_eq!(
            restored.binding(Action::Next),
            Keymap::default().binding(Action::Next),
            "one bad entry disturbed the others"
        );
    }

    #[test]
    fn every_modifier_combination_round_trips_as_text() {
        for (ctrl, shift, alt) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (false, false, true),
            (true, true, true),
        ] {
            let binding = KeyBinding {
                ctrl,
                shift,
                alt,
                code: key::Code::KeyK,
            };
            assert_eq!(KeyBinding::parse(&binding.stored()), Some(binding));
        }
    }

    #[test]
    fn a_bindable_key_always_survives_being_stored() {
        for (code, _) in CODE_NAMES {
            assert!(
                is_bindable(*code),
                "{} is spelled but not bindable",
                code_name(*code)
            );
            let binding = KeyBinding::plain(*code);
            assert_eq!(
                KeyBinding::parse(&binding.stored()),
                Some(binding),
                "{} could be bound but not read back",
                code_name(*code)
            );
        }
    }

    #[test]
    fn every_named_code_round_trips_as_text() {
        for (code, _) in CODE_NAMES {
            let binding = KeyBinding::plain(*code);
            assert_eq!(
                KeyBinding::parse(&binding.stored()),
                Some(binding),
                "{} did not survive",
                code_name(*code)
            );
        }
    }

    #[test]
    fn a_key_outside_the_table_cannot_be_bound() {
        assert!(!is_bindable(key::Code::F13));
        assert!(!is_bindable(key::Code::Numpad0));
        assert!(is_bindable(key::Code::KeyA));
    }

    #[test]
    fn junk_text_parses_to_nothing() {
        for text in ["", "Ctrl+", "NotAKey", "Ctrl+A+B", "Hyper+A"] {
            assert_eq!(KeyBinding::parse(text), None, "{text:?} parsed");
        }
    }

    #[test]
    fn actions_have_distinct_storage_keys() {
        let mut keys: Vec<String> = Action::all().into_iter().map(Action::key).collect();
        keys.sort();
        let total = keys.len();
        keys.dedup();

        assert_eq!(keys.len(), total, "two actions serialize to the same name");
    }

    #[test]
    fn a_modifier_key_is_not_bindable_on_its_own() {
        assert!(is_modifier(key::Code::ControlLeft));
        assert!(!is_modifier(key::Code::KeyA));
    }
}
