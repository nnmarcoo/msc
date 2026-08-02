//! The pane options modal: everything about one pane that is a setting.
//!
//! A pane's *actions* — grab, split, close — stay in the edit overlay, where
//! they are one click away because they are what you repeat while arranging a
//! layout. Its *settings* live here: the kind it shows, the lock on its size,
//! and whatever the kind itself has to configure.
//!
//! This is a modal over a scrim rather than a panel hanging off the gear. A
//! dropdown is the right shape for picking one value from a list and the wrong
//! one for a form: it is tethered to its trigger, so it cannot be wider than the
//! corner it grows from, and it dismisses on any stray click, which is a hazard
//! rather than a convenience once there is something to fill in. The scrim also
//! says plainly that the rest of the window is not what you are editing.
//!
//! It is deliberately not a separate window either. These settings describe one
//! pane, so they must not outlive it; a window could drift behind the app, move
//! to another monitor, or stay open after its pane was closed, and every one of
//! those needs an answer that a modal simply does not raise. Closing the pane
//! closes this, because [`crate::app`] holds the open pane's id and the modal is
//! drawn only while that id is still in the layout.
//!
//! Unlike [`crate::preferences`], which replaces the pane tree outright, this
//! draws *over* the panes via `stack!`. Preferences is a mode the window enters;
//! this is a question asked about something still on screen, and hiding the pane
//! being configured would take away the thing the settings refer to.
//!
//! Settings apply immediately rather than on a Save button. Every one of them is
//! visible in the pane behind the scrim, so the edit and its result are on
//! screen together, and a dialog that made you commit before seeing the outcome
//! would be worse for exactly the settings this exists to hold. That is why
//! there is no Cancel: with nothing batched, there is nothing to roll back.
//!
//! [`section`] and [`setting`] borrow their shape from [`crate::preferences`]
//! deliberately: the same semibold group heading, the same title-over-
//! description label block, the same [`HoverRow`] and spacing. Settings are
//! settings wherever they are met, and a dialog that invented its own row would
//! read as a different application's. Adding a setting is a line rather than a
//! new arrangement, which is what keeps that true as this grows per-kind
//! sections. The choices are `pick_list`s for the same reason, styled as the
//! theme picker is; a segmented row of buttons read as a toolbar rather than as
//! one value being named, and grew with the option count where a trigger does
//! not.
//!
//! The title *is* the kind picker. The kind is what the whole dialog is about,
//! so naming it in the header and again in a "Type" row said the same thing
//! twice while only the row could change it. Only the kind on screen contributes
//! a section below, so the dialog holds the settings for what the pane *is*
//! rather than for everything it has been.
//!
//! The backdrop dismisses on press, and the dialog is wrapped in its own
//! `mouse_area` so a press inside it is swallowed rather than reaching the
//! backdrop and closing what the user is using. [`Scrim`] then claims the
//! *cursor* over the whole backdrop, since a `mouse_area` intercepts presses
//! only — without it the panes underneath keep hovering their rows through it.
//!
//! The lock cycles rather than picks, because locking freezes the pane's
//! *measured* size at the moment it is set: there is no list of values to choose
//! from, only the current shape to hold. Its icon and its words are shown
//! together — the three locked glyphs differ only in which edges they mark,
//! which is easy to miss and impossible to guess the first time.

use iced::alignment::Vertical;
use iced::font::Weight;
use iced::widget::svg::Handle;
use iced::widget::{
    Space, button, column, container, mouse_area, pick_list, row as row_widget, scrollable, svg,
    text, toggler,
};
use iced::{Element, Font, Length, Theme};

use crate::app::Message;
use crate::layout::{Locks, PaneId};
use crate::pane::PaneKind;
use crate::pane::settings::{
    Accent, Caps, Density, PaneSettings, Settings, Timeline as TimelineSettings, Tint,
    TrackInfo as TrackInfoSettings, Visualizer, Volume as VolumeSettings,
};
use crate::styles::{self, PAD, muted_text};
use crate::widgets::hover_row::HoverRow;
use crate::widgets::pane_picker::PanePicker;
use crate::widgets::scrim::Scrim;
use crate::widgets::tooltip::tip;

const ICON_LOCK_WIDTH: &[u8] = include_bytes!("../../../assets/icons/lock_width.svg");
const ICON_LOCK_HEIGHT: &[u8] = include_bytes!("../../../assets/icons/lock_height.svg");
const ICON_LOCK_BOTH: &[u8] = include_bytes!("../../../assets/icons/lock_both.svg");
const ICON_UNLOCK: &[u8] = include_bytes!("../../../assets/icons/unlock.svg");
const ICON_CLOSE: &[u8] = include_bytes!("../../../assets/icons/close.svg");
const ICON_SETTINGS: &[u8] = include_bytes!("../../../assets/icons/settings.svg");

const MODAL_WIDTH: f32 = 420.0;
const MODAL_MAX_HEIGHT: f32 = 560.0;

const TITLE_SIZE: f32 = 15.0;
const HEADING_SIZE: f32 = 14.0;
const LABEL_SIZE: f32 = 13.0;
const DESCRIPTION_SIZE: f32 = 11.0;

const ICON_SIZE: f32 = 14.0;

const CLOSE_ICON_SIZE: f32 = 18.0;

const ROW_GAP: f32 = PAD * 3.0;
const GAP: f32 = PAD * 2.0;
const PADDING: f32 = PAD * 4.0;

pub fn view<'a>(
    id: PaneId,
    kind: PaneKind,
    locks: Locks,
    settings: &PaneSettings,
) -> Element<'a, Message> {
    let mut groups = column![pane_section(id, locks)].spacing(GAP * 2.0);

    if let Some(section) = kind_section(id, kind, settings) {
        groups = groups.push(section);
    }

    let body = column![
        title_bar(id, kind),
        section_rule(),
        scrollable(groups)
            .height(Length::Shrink)
            .width(Length::Fill),
    ]
    .spacing(GAP);

    let dialog = container(body)
        .padding(PADDING)
        .width(Length::Fixed(MODAL_WIDTH))
        .max_height(MODAL_MAX_HEIGHT)
        .style(styles::modal_style);

    Scrim::new(
        mouse_area(
            container(mouse_area(dialog).on_press(Message::Noop))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(styles::scrim_style),
        )
        .on_press(Message::ClosePaneOptions),
    )
    .into()
}

fn title_bar<'a>(id: PaneId, kind: PaneKind) -> Element<'a, Message> {
    let heading = row_widget![
        icon(ICON_SETTINGS, TITLE_SIZE),
        tip(
            PanePicker::new(kind, move |picked| Message::SetPaneKind(id, picked))
                .text_size(TITLE_SIZE),
            "Change pane type",
        ),
    ]
    .spacing(PAD)
    .align_y(Vertical::Center);

    row_widget![
        heading,
        Space::new().width(Length::Fill),
        tip(
            button(icon(ICON_CLOSE, CLOSE_ICON_SIZE))
                .padding(PAD)
                .style(styles::icon_button_style)
                .on_press(Message::ClosePaneOptions),
            "Close",
        ),
    ]
    .align_y(Vertical::Center)
    .into()
}

fn pane_section<'a>(id: PaneId, locks: Locks) -> Element<'a, Message> {
    section(
        "Pane",
        vec![setting(
            "Size",
            "Hold this pane's width or height when the window resizes",
            lock_control(id, locks),
        )],
    )
}

fn kind_section<'a>(
    id: PaneId,
    kind: PaneKind,
    settings: &PaneSettings,
) -> Option<Element<'a, Message>> {
    match kind {
        PaneKind::Visualizer => Some(visualizer_section(id, settings.visualizer())),
        PaneKind::Timeline => {
            let current = settings.timeline();
            Some(accent_section(
                "Timeline",
                "What colors the seek bar and the title",
                current.accent,
                move |accent| Settings::Timeline(TimelineSettings { accent }),
                id,
            ))
        }
        PaneKind::Volume => {
            let current = settings.volume();
            Some(accent_section(
                "Volume",
                "What colors the filled part of the rail",
                current.accent,
                move |accent| Settings::Volume(VolumeSettings { accent }),
                id,
            ))
        }
        PaneKind::TrackInfo => {
            let current = settings.track_info();
            Some(accent_section(
                "Track information",
                "What colors the track's title",
                current.accent,
                move |accent| Settings::TrackInfo(TrackInfoSettings { accent }),
                id,
            ))
        }
        _ => None,
    }
}

fn accent_section<'a>(
    heading: &'a str,
    description: &'a str,
    current: Accent,
    into_settings: impl Fn(Accent) -> Settings + 'a,
    id: PaneId,
) -> Element<'a, Message> {
    section(
        heading,
        vec![setting(
            "Color",
            description,
            choices(&Accent::ALL, current, move |next| {
                Message::SetPaneSettings(id, into_settings(next))
            }),
        )],
    )
}

fn visualizer_section<'a>(id: PaneId, current: Visualizer) -> Element<'a, Message> {
    let update = move |next: Visualizer| Message::SetPaneSettings(id, Settings::Visualizer(next));

    section(
        "Spectrum",
        vec![
            setting(
                "Density",
                "How many bars the pane fits across its width",
                choices(&Density::ALL, current.density, move |next| {
                    update(Visualizer {
                        density: next,
                        ..current
                    })
                }),
            ),
            setting(
                "Caps",
                "How much the top of each bar is rounded",
                choices(&Caps::ALL, current.caps, move |next| {
                    update(Visualizer {
                        caps: next,
                        ..current
                    })
                }),
            ),
            setting(
                "Color",
                "What decides each bar's color",
                choices(&Tint::ALL, current.tint, move |next| {
                    update(Visualizer {
                        tint: next,
                        ..current
                    })
                }),
            ),
            setting(
                "Peak hold",
                "Leave a falling marker at each band's recent peak",
                toggler(current.peak_hold)
                    .on_toggle(move |peak_hold| {
                        update(Visualizer {
                            peak_hold,
                            ..current
                        })
                    })
                    .into(),
            ),
        ],
    )
}

fn choices<'a, T>(
    all: &'a [T],
    current: T,
    on_pick: impl Fn(T) -> Message + 'a,
) -> Element<'a, Message>
where
    T: Copy + Eq + std::fmt::Display + 'static,
{
    pick_list(all, Some(current), on_pick)
        .text_size(DESCRIPTION_SIZE)
        .padding([PAD / 2.0, PAD])
        .style(styles::pick_list_style)
        .menu_style(styles::pick_list_menu_style)
        .into()
}

fn lock_control<'a>(id: PaneId, locks: Locks) -> Element<'a, Message> {
    let (glyph, label) = lock_glyph(locks);

    button(
        row_widget![icon(glyph, ICON_SIZE), text(label).size(DESCRIPTION_SIZE)]
            .spacing(PAD)
            .align_y(Vertical::Center),
    )
    .padding([PAD, PAD * 2.0])
    .style(styles::modal_control_style)
    .on_press(Message::CyclePaneLock(id))
    .into()
}

pub fn lock_glyph(locks: Locks) -> (&'static [u8], &'static str) {
    match (locks.width, locks.height) {
        (None, None) => (ICON_UNLOCK, "Free"),
        (Some(_), None) => (ICON_LOCK_WIDTH, "Width locked"),
        (None, Some(_)) => (ICON_LOCK_HEIGHT, "Height locked"),
        (Some(_), Some(_)) => (ICON_LOCK_BOTH, "Width and height locked"),
    }
}

fn icon<'a>(bytes: &'static [u8], size: f32) -> Element<'a, Message> {
    svg(Handle::from_memory(bytes))
        .style(styles::svg_style)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

fn section<'a>(heading: &'a str, rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut list = column![].spacing(ROW_GAP).width(Length::Fill);
    for entry in rows {
        list = list.push(entry);
    }

    column![
        text(heading).size(HEADING_SIZE).font(Font {
            weight: Weight::Semibold,
            ..Font::DEFAULT
        }),
        list,
    ]
    .spacing(GAP)
    .width(Length::Fill)
    .into()
}

fn setting<'a>(
    label: &'a str,
    description: &'a str,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    HoverRow::new(
        row_widget![label_block(label, description), control]
            .align_y(Vertical::Center)
            .spacing(PAD * 2.0),
    )
    .into()
}

fn label_block<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    let block = column![
        text(title).size(LABEL_SIZE),
        text(description)
            .size(DESCRIPTION_SIZE)
            .style(|theme: &Theme| text::Style {
                color: Some(muted_text(theme))
            }),
    ]
    .spacing(PAD / 2.0);

    container(block).clip(true).width(Length::Fill).into()
}

fn section_rule<'a>() -> Element<'a, Message> {
    container(Space::new().height(1.0))
        .width(Length::Fill)
        .style(styles::pref_rule_style)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock_label(locks: Locks) -> &'static str {
        lock_glyph(locks).1
    }

    #[test]
    fn every_lock_state_has_its_own_icon() {
        let states = [
            Locks::default(),
            Locks {
                width: Some(320.0),
                height: None,
            },
            Locks {
                width: None,
                height: Some(180.0),
            },
            Locks {
                width: Some(320.0),
                height: Some(180.0),
            },
        ];

        for (i, a) in states.iter().enumerate() {
            for b in &states[i + 1..] {
                assert_ne!(
                    lock_glyph(*a).0.as_ptr(),
                    lock_glyph(*b).0.as_ptr(),
                    "{:?} and {:?} share an icon",
                    lock_label(*a),
                    lock_label(*b)
                );
            }
        }
    }

    #[test]
    fn the_lock_label_names_every_state() {
        assert_eq!(lock_label(Locks::default()), "Free");
        assert_eq!(
            lock_label(Locks {
                width: Some(320.0),
                height: None
            }),
            "Width locked"
        );
        assert_eq!(
            lock_label(Locks {
                width: None,
                height: Some(180.0)
            }),
            "Height locked"
        );
        assert_eq!(
            lock_label(Locks {
                width: Some(320.0),
                height: Some(180.0)
            }),
            "Width and height locked"
        );
    }
}
