//! The preferences view: a full-window mode that replaces the pane tree.
//!
//! This is deliberately not a pane. Panes are content the layout arranges, and
//! their whole point is that the user chooses where they sit and how many there
//! are; preferences are a mode the window enters and leaves, and a second copy
//! of them open beside the first would be a bug rather than a feature. So
//! [`crate::app::App::view`] returns this *instead of* the panes whenever a
//! pending config exists, and `s` toggles it the way `e` toggles edit mode.
//!
//! Edits are made against a clone of the config rather than the live one, which
//! is what lets the footer offer a real Cancel. `Save` hands the clone back to
//! the app to commit and write; `Cancel` drops it and nothing was touched.
//!
//! Both appearance settings preview live, so what the dialog shows is what Save
//! keeps. The theme does it by [`crate::app::App::theme`] reading the pending
//! config when one exists, since iced asks for the theme every frame. The corner
//! radius cannot work that way: it lives in a global that styling reads during
//! layout, so setting it is the only way to show it, and that write outlives a
//! Cancel. Cancel therefore restores the radius from the live config — the one
//! edit here with anything to undo.
//!
//! The section list is an enum rather than an index, so adding a section cannot
//! silently renumber the others, and [`PrefSection::About`] is pinned to the
//! bottom of the sidebar because it is not a setting.
//!
//! `Keybindings` and `Playback` sections are expected here; the layout is built
//! from `subgroup` and `setting` so that adding one is a list of rows rather
//! than a new arrangement.
//!
//! Neither reset touches `layouts` or `active_layout`, which is why `ResetAll`
//! names its fields rather than assigning a whole `Config::default()`. Panes the
//! user arranged are work, not a setting that drifted, and no arrangement is
//! reachable from this view to undo; wiping them from a button labelled "Reset
//! all settings" would destroy the most expensive thing in the file.

use iced::alignment::{Horizontal, Vertical};
use iced::font::Weight;
use iced::keyboard::{
    self,
    key::{self, Physical},
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg::Handle;
use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, svg, text, toggler,
};
use iced::{Color, Element, Font, Length, Theme};

use crate::app::Message;
use crate::config::{ALL_THEMES, Config};
use crate::keybinds::{self, Action, KeyBinding, KeyCategory, Keymap};
use crate::styles::{
    self, BAR_HEIGHT, PAD, PREF_CONTENT_MAX_WIDTH, PREF_SIDEBAR_WIDTH, RULE_HEIGHT, muted_text,
    set_radius,
};
use crate::widgets::hover_row::HoverRow;
use crate::widgets::tooltip::tip;

const ICON_CHECK: &[u8] = include_bytes!("../../assets/icons/check.svg");
const ICON_CLOSE: &[u8] = include_bytes!("../../assets/icons/close.svg");

const ICON_SIZE: f32 = 16.0;

const CLEAR_SLOT: f32 = ICON_SIZE + PAD * 4.0;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefSection {
    #[default]
    Appearance,
    Keybindings,
    About,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub winner: Action,
    pub losers: Vec<Action>,
    pub binding: KeyBinding,
}

#[derive(Default)]
pub struct PreferenceState {
    pub section: PrefSection,
    pub capturing: Option<Action>,
    pub conflict: Option<Conflict>,
}

#[derive(Debug, Clone)]
pub enum PreferenceMessage {
    SelectSection(PrefSection),
    SetTheme(Theme),
    SetRounded(bool),
    StartCapture(Action),
    CancelCapture,
    SetBinding(Action, KeyBinding),
    ClearBinding(Action),
    ResetAppearance,
    ResetKeybindings,
    ResetAll,
    Save,
    Cancel,
}

pub fn capture_key(
    state: &PreferenceState,
    physical: Physical,
    modifiers: keyboard::Modifiers,
) -> Option<PreferenceMessage> {
    let action = state.capturing?;
    let Physical::Code(code) = physical else {
        return None;
    };

    if keybinds::is_modifier(code) {
        return None;
    }
    if code == key::Code::Escape {
        return Some(PreferenceMessage::CancelCapture);
    }
    if code == key::Code::Backspace && modifiers.is_empty() {
        return Some(PreferenceMessage::ClearBinding(action));
    }
    if !keybinds::is_bindable(code) {
        return None;
    }

    Some(PreferenceMessage::SetBinding(
        action,
        KeyBinding {
            ctrl: modifiers.control(),
            shift: modifiers.shift(),
            alt: modifiers.alt(),
            code,
        },
    ))
}

pub enum PreferenceOutcome {
    Open,
    Save,
    Cancel,
}

pub fn update(
    message: PreferenceMessage,
    pending: &mut Config,
    state: &mut PreferenceState,
) -> PreferenceOutcome {
    state.conflict = None;

    match message {
        PreferenceMessage::SelectSection(section) => {
            state.section = section;
            state.capturing = None;
            PreferenceOutcome::Open
        }
        PreferenceMessage::StartCapture(action) => {
            state.capturing = Some(action);
            PreferenceOutcome::Open
        }
        PreferenceMessage::CancelCapture => {
            state.capturing = None;
            PreferenceOutcome::Open
        }
        PreferenceMessage::SetBinding(action, binding) => {
            let losers = pending.keymap.set(action, binding);
            if !losers.is_empty() {
                state.conflict = Some(Conflict {
                    winner: action,
                    losers,
                    binding,
                });
            }
            state.capturing = None;
            PreferenceOutcome::Open
        }
        PreferenceMessage::ClearBinding(action) => {
            pending.keymap.clear(action);
            state.capturing = None;
            PreferenceOutcome::Open
        }
        PreferenceMessage::ResetKeybindings => {
            pending.keymap = Keymap::default();
            state.capturing = None;
            PreferenceOutcome::Open
        }
        PreferenceMessage::SetTheme(theme) => {
            pending.theme = theme;
            PreferenceOutcome::Open
        }
        PreferenceMessage::SetRounded(rounded) => {
            pending.rounded = rounded;
            set_radius(rounded);
            PreferenceOutcome::Open
        }
        PreferenceMessage::ResetAppearance => {
            let defaults = Config::default();
            pending.theme = defaults.theme;
            pending.rounded = defaults.rounded;
            set_radius(pending.rounded);
            PreferenceOutcome::Open
        }
        PreferenceMessage::ResetAll => {
            let defaults = Config::default();
            pending.theme = defaults.theme;
            pending.rounded = defaults.rounded;
            pending.volume = defaults.volume;
            pending.muted = defaults.muted;
            pending.keymap = defaults.keymap;
            set_radius(pending.rounded);
            state.capturing = None;
            PreferenceOutcome::Open
        }
        PreferenceMessage::Save => {
            set_radius(pending.rounded);
            PreferenceOutcome::Save
        }
        PreferenceMessage::Cancel => PreferenceOutcome::Cancel,
    }
}

fn label_block<'a>(
    title: impl text::IntoFragment<'a>,
    description: impl text::IntoFragment<'a>,
    note: Option<(String, Color)>,
    theme: &Theme,
) -> Element<'a, Message> {
    let mut block = column![
        text(title).size(13),
        text(description).size(11).color(muted_text(theme)),
    ]
    .spacing(PAD / 2.0);

    if let Some((note, color)) = note {
        block = block.push(text(note).size(11).color(color));
    }

    container(block).clip(true).width(Length::Fill).into()
}

fn setting<'a>(
    label: &'a str,
    description: &'a str,
    control: Element<'a, Message>,
    theme: &Theme,
) -> Element<'a, Message> {
    HoverRow::new(
        row![label_block(label, description, None, theme), control]
            .align_y(Vertical::Center)
            .spacing(PAD * 2.0),
    )
    .into()
}

fn keybind_row<'a>(
    action: Action,
    keymap: &Keymap,
    state: &PreferenceState,
    theme: &Theme,
) -> Element<'a, Message> {
    let capturing = state.capturing == Some(action);
    let binding = keymap.binding(action);

    let chip: Element<'a, Message> = if capturing {
        button(text("Press a key\u{2026}").size(11))
            .style(styles::capturing_chip_style)
            .on_press(Message::Preference(PreferenceMessage::CancelCapture))
            .padding([4.0, 8.0])
            .into()
    } else {
        tip(
            button(text(binding.map_or_else(|| "\u{2014}".into(), KeyBinding::shown)).size(11))
                .style(styles::key_chip_style)
                .on_press(Message::Preference(PreferenceMessage::StartCapture(action)))
                .padding([4.0, 8.0]),
            "Set a key for this action",
        )
        .into()
    };

    let note = if capturing {
        let hint = if binding.is_some() {
            "Backspace removes the key, Esc cancels"
        } else {
            "Esc cancels"
        };
        Some((hint.into(), muted_text(theme)))
    } else {
        state
            .conflict
            .as_ref()
            .filter(|conflict| conflict.winner == action)
            .map(|conflict| {
                let losers = conflict
                    .losers
                    .iter()
                    .map(|action| action.label())
                    .collect::<Vec<_>>()
                    .join(", ");
                (
                    format!("{} was taken from {losers}", conflict.binding.shown()),
                    theme.extended_palette().danger.base.color,
                )
            })
    };

    let clear: Option<Element<'a, Message>> = (binding.is_some() && !capturing).then(|| {
        icon_button(
            ICON_CLOSE,
            "Remove this key",
            Message::Preference(PreferenceMessage::ClearBinding(action)),
        )
    });

    HoverRow::new(label_block(
        action.label(),
        action.description(),
        note,
        theme,
    ))
    .trailing(chip)
    .hover_slot(CLEAR_SLOT, clear)
    .into()
}

fn subgroup<'a>(
    label: &'a str,
    reset: Option<(&'a str, PreferenceMessage)>,
    rows: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let heading = text(label).size(14).font(Font {
        weight: Weight::Semibold,
        ..Font::DEFAULT
    });

    let header: Element<'a, Message> = match reset {
        Some((label, on_reset)) => row![
            heading,
            Space::new().width(Length::Fill),
            tip(
                button(text("Reset").size(11))
                    .style(styles::icon_button_style)
                    .on_press(Message::Preference(on_reset))
                    .padding([2.0, 6.0]),
                label,
            ),
        ]
        .align_y(Vertical::Center)
        .into(),
        None => heading.into(),
    };

    let mut list = column![].spacing(PAD * 3.0).width(Length::Fill);
    for row in rows {
        list = list.push(row);
    }

    column![column![header, section_rule()].spacing(PAD), list]
        .spacing(PAD * 2.0)
        .width(Length::Fill)
        .into()
}

fn nav_button(label: &str, target: PrefSection, active: bool) -> Element<'_, Message> {
    button(text(label).size(13))
        .width(Length::Fill)
        .padding([6.0, 8.0])
        .style(styles::pref_nav_button_style(active))
        .on_press(Message::Preference(PreferenceMessage::SelectSection(
            target,
        )))
        .into()
}

fn appearance<'a>(pending: &'a Config, theme: &Theme) -> Element<'a, Message> {
    let rows = vec![
        setting(
            "Theme",
            "Colour scheme for the application",
            pick_list(ALL_THEMES, Some(pending.theme.clone()), |theme| {
                Message::Preference(PreferenceMessage::SetTheme(theme))
            })
            .text_size(12)
            .style(styles::pick_list_style)
            .menu_style(styles::pick_list_menu_style)
            .into(),
            theme,
        ),
        setting(
            "Rounded corners",
            "Round the corners of buttons, panels, and panes",
            toggler(pending.rounded)
                .on_toggle(|rounded| Message::Preference(PreferenceMessage::SetRounded(rounded)))
                .into(),
            theme,
        ),
    ];

    subgroup(
        "Appearance",
        Some((
            "Reset appearance to defaults",
            PreferenceMessage::ResetAppearance,
        )),
        rows,
    )
}

fn keybindings<'a>(
    pending: &'a Config,
    state: &PreferenceState,
    theme: &Theme,
) -> Element<'a, Message> {
    let actions = Action::all();
    let mut sections = column![].spacing(PAD * 5.0).width(Length::Fill);
    let mut first = true;

    for &category in KeyCategory::all() {
        let rows: Vec<Element<'a, Message>> = actions
            .iter()
            .filter(|action| action.category() == category)
            .map(|&action| keybind_row(action, &pending.keymap, state, theme))
            .collect();

        if rows.is_empty() {
            continue;
        }

        let reset = first.then_some((
            "Reset every key to its default",
            PreferenceMessage::ResetKeybindings,
        ));
        sections = sections.push(subgroup(category.label(), reset, rows));
        first = false;
    }

    sections.into()
}

fn about<'a>(theme: &Theme) -> Element<'a, Message> {
    let muted = muted_text(theme);

    column![
        text("Verse").size(24).font(Font {
            weight: Weight::Semibold,
            ..Font::DEFAULT
        }),
        text(concat!("Version ", env!("CARGO_PKG_VERSION")))
            .size(12)
            .color(muted),
        Space::new().height(PAD),
        text(env!("CARGO_PKG_DESCRIPTION")).size(13),
        Space::new().height(PAD * 2.0),
        text(concat!("Licensed under ", env!("CARGO_PKG_LICENSE")))
            .size(11)
            .color(muted),
    ]
    .spacing(PAD)
    .align_x(Horizontal::Center)
    .width(Length::Fill)
    .into()
}

fn divider<'a>() -> Element<'a, Message> {
    container(Space::new().height(RULE_HEIGHT))
        .width(Length::Fill)
        .style(styles::pref_divider_style)
        .into()
}

fn sidebar_edge<'a>() -> Element<'a, Message> {
    container(Space::new().width(RULE_HEIGHT))
        .height(Length::Fill)
        .style(styles::pref_divider_style)
        .into()
}

fn section_rule<'a>() -> Element<'a, Message> {
    container(Space::new().height(1.0))
        .width(Length::Fill)
        .style(styles::pref_rule_style)
        .into()
}

fn bar<'a>(content: impl Into<Element<'a, Message>>, divider_on_top: bool) -> Element<'a, Message> {
    let body = container(content)
        .width(Length::Fill)
        .height(Length::Fixed(BAR_HEIGHT))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .padding([0.0, PAD]);

    let stack = if divider_on_top {
        column![divider(), body]
    } else {
        column![body, divider()]
    }
    .width(Length::Fill);

    container(stack)
        .width(Length::Fill)
        .style(styles::bar_style)
        .into()
}

fn icon_button<'a>(bytes: &'static [u8], label: &'a str, message: Message) -> Element<'a, Message> {
    let glyph = svg(Handle::from_memory(bytes))
        .style(styles::svg_style)
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE));

    tip(
        button(glyph)
            .on_press(message)
            .padding(PAD)
            .style(styles::icon_button_style),
        label,
    )
    .into()
}

pub fn view<'a>(pending: &'a Config, state: &'a PreferenceState) -> Element<'a, Message> {
    let theme = &pending.theme;
    let active = state.section;

    let sidebar = container(
        column![
            nav_button(
                "Appearance",
                PrefSection::Appearance,
                active == PrefSection::Appearance
            ),
            nav_button(
                "Keybindings",
                PrefSection::Keybindings,
                active == PrefSection::Keybindings
            ),
            Space::new().height(Length::Fill),
            nav_button("About", PrefSection::About, active == PrefSection::About),
        ]
        .spacing(PAD)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fixed(PREF_SIDEBAR_WIDTH))
    .height(Length::Fill)
    .padding(PAD * 2.0);

    let section = match active {
        PrefSection::Appearance => appearance(pending, theme),
        PrefSection::Keybindings => keybindings(pending, state, theme),
        PrefSection::About => about(theme),
    };

    let content: Element<'a, Message> = if active == PrefSection::About {
        container(section)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .padding(PAD * 3.0)
            .into()
    } else {
        scrollable(
            container(section)
                .max_width(PREF_CONTENT_MAX_WIDTH)
                .width(Length::Fill)
                .padding(PAD * 3.0),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(Direction::Vertical(
            Scrollbar::new().width(4).scroller_width(4),
        ))
        .into()
    };

    let footer = bar(
        row![
            tip(
                button(text("Reset all").size(12))
                    .style(styles::icon_button_style)
                    .on_press(Message::Preference(PreferenceMessage::ResetAll))
                    .padding([4.0, 8.0]),
                "Reset all settings to defaults",
            ),
            Space::new().width(Length::Fill),
            icon_button(
                ICON_CHECK,
                "Save",
                Message::Preference(PreferenceMessage::Save)
            ),
            icon_button(
                ICON_CLOSE,
                "Cancel",
                Message::Preference(PreferenceMessage::Cancel)
            ),
        ]
        .width(Length::Fill)
        .align_y(Vertical::Center)
        .spacing(PAD),
        true,
    );

    column![
        bar(text("Preferences").size(16), false),
        row![sidebar, sidebar_edge(), content]
            .width(Length::Fill)
            .height(Length::Fill),
        footer,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Layout;
    use crate::pane::PaneKind;

    fn edited() -> Config {
        Config {
            theme: Theme::Nord,
            rounded: false,
            ..Config::default()
        }
    }

    fn apply(message: PreferenceMessage, pending: &mut Config) -> PreferenceOutcome {
        update(message, pending, &mut PreferenceState::default())
    }

    #[test]
    fn an_edit_keeps_the_dialog_open() {
        let mut pending = Config::default();
        let outcome = apply(PreferenceMessage::SetTheme(Theme::Nord), &mut pending);

        assert!(matches!(outcome, PreferenceOutcome::Open));
        assert_eq!(pending.theme, Theme::Nord);
    }

    #[test]
    fn saving_reports_a_save_and_cancelling_a_cancel() {
        let mut pending = Config::default();

        assert!(matches!(
            apply(PreferenceMessage::Save, &mut pending),
            PreferenceOutcome::Save
        ));
        assert!(matches!(
            apply(PreferenceMessage::Cancel, &mut pending),
            PreferenceOutcome::Cancel
        ));
    }

    #[test]
    fn cancelling_leaves_the_pending_edits_alone() {
        let mut pending = edited();
        apply(PreferenceMessage::Cancel, &mut pending);

        assert_eq!(pending.theme, Theme::Nord);
        assert!(!pending.rounded);
    }

    #[test]
    fn resetting_appearance_restores_both_of_its_settings() {
        let defaults = Config::default();
        let mut pending = edited();
        apply(PreferenceMessage::ResetAppearance, &mut pending);

        assert_eq!(pending.theme, defaults.theme);
        assert_eq!(pending.rounded, defaults.rounded);
    }

    #[test]
    fn resetting_everything_restores_the_playback_settings_too() {
        let defaults = Config::default();
        let mut pending = edited();
        pending.volume = 0.9;
        pending.muted = true;

        apply(PreferenceMessage::ResetAll, &mut pending);

        assert_eq!(pending.theme, defaults.theme);
        assert_eq!(pending.rounded, defaults.rounded);
        assert_eq!(pending.volume.to_bits(), defaults.volume.to_bits());
        assert_eq!(pending.muted, defaults.muted);
    }

    #[test]
    fn no_reset_discards_the_layouts_the_user_built() {
        let mine = vec![Layout::single("Mine", PaneKind::Queue)];

        for message in [
            PreferenceMessage::ResetAppearance,
            PreferenceMessage::ResetAll,
        ] {
            let mut pending = Config {
                layouts: mine.clone(),
                active_layout: 0,
                ..edited()
            };
            apply(message, &mut pending);

            assert_eq!(pending.layouts, mine, "a reset wiped the saved layouts");
        }
    }

    fn capturing(action: Action) -> PreferenceState {
        PreferenceState {
            capturing: Some(action),
            ..PreferenceState::default()
        }
    }

    fn press(code: key::Code) -> Physical {
        Physical::Code(code)
    }

    #[test]
    fn a_key_pressed_outside_a_capture_is_not_swallowed() {
        let state = PreferenceState::default();
        assert!(
            capture_key(&state, press(key::Code::KeyA), keyboard::Modifiers::empty()).is_none(),
            "a key was captured with no row waiting for one"
        );
    }

    #[test]
    fn a_captured_key_becomes_the_binding() {
        let state = capturing(Action::Shuffle);
        let message = capture_key(&state, press(key::Code::KeyJ), keyboard::Modifiers::CTRL);

        match message {
            Some(PreferenceMessage::SetBinding(action, binding)) => {
                assert_eq!(action, Action::Shuffle);
                assert_eq!(binding.code, key::Code::KeyJ);
                assert!(binding.ctrl);
                assert!(!binding.shift);
            }
            other => panic!("expected a binding, got {other:?}"),
        }
    }

    #[test]
    fn escape_cancels_and_backspace_unbinds() {
        let state = capturing(Action::Shuffle);
        let empty = keyboard::Modifiers::empty();

        assert!(matches!(
            capture_key(&state, press(key::Code::Escape), empty),
            Some(PreferenceMessage::CancelCapture)
        ));
        assert!(matches!(
            capture_key(&state, press(key::Code::Backspace), empty),
            Some(PreferenceMessage::ClearBinding(Action::Shuffle))
        ));
    }

    #[test]
    fn a_modified_backspace_is_a_binding_rather_than_an_unbind() {
        let state = capturing(Action::Shuffle);
        let message = capture_key(
            &state,
            press(key::Code::Backspace),
            keyboard::Modifiers::CTRL,
        );

        assert!(
            matches!(message, Some(PreferenceMessage::SetBinding(..))),
            "Ctrl+Backspace unbound the action instead of binding to it"
        );
    }

    #[test]
    fn a_modifier_alone_never_becomes_a_binding() {
        let state = capturing(Action::Shuffle);
        for code in [key::Code::ControlLeft, key::Code::ShiftRight] {
            assert!(
                capture_key(&state, press(code), keyboard::Modifiers::empty()).is_none(),
                "a modifier was captured as a key"
            );
        }
    }

    #[test]
    fn a_key_that_cannot_be_stored_is_refused_at_capture() {
        let state = capturing(Action::Shuffle);
        let message = capture_key(&state, press(key::Code::F13), keyboard::Modifiers::empty());

        assert!(
            message.is_none(),
            "a key with no stored spelling was accepted, and would vanish on restart"
        );
    }

    #[test]
    fn a_conflict_is_reported_and_then_cleared_by_the_next_edit() {
        let mut pending = Config::default();
        let mut state = PreferenceState::default();
        let space = KeyBinding::plain(key::Code::Space);

        update(
            PreferenceMessage::SetBinding(Action::Shuffle, space),
            &mut pending,
            &mut state,
        );

        let conflict = state.conflict.as_ref().expect("a reported conflict");
        assert_eq!(conflict.winner, Action::Shuffle);
        assert_eq!(conflict.losers, vec![Action::PlayPause]);

        update(
            PreferenceMessage::SelectSection(PrefSection::About),
            &mut pending,
            &mut state,
        );
        assert!(
            state.conflict.is_none(),
            "the conflict note outlived its edit"
        );
    }

    #[test]
    fn resetting_keybindings_restores_the_defaults() {
        let mut pending = Config::default();
        pending.keymap.clear(Action::PlayPause);

        apply(PreferenceMessage::ResetKeybindings, &mut pending);

        assert_eq!(pending.keymap, Keymap::default());
    }

    #[test]
    fn selecting_a_section_moves_the_sidebar() {
        let mut pending = Config::default();
        let mut state = PreferenceState::default();
        assert_eq!(state.section, PrefSection::Appearance);

        update(
            PreferenceMessage::SelectSection(PrefSection::About),
            &mut pending,
            &mut state,
        );

        assert_eq!(state.section, PrefSection::About);
    }
}
