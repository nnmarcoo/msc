//! Per-pane settings: the choices about a pane that are taste rather than
//! consequence.
//!
//! A setting belongs here only when two panes of the same kind could reasonably
//! disagree about it and neither answer is derivable. Anything a pane can work
//! out from its own measured size stays derived — [`crate::pane::volume::Form`]
//! and the spectrum's bar count both read the bounds they were given — because
//! making those settings would let a pane be configured compact at a width where
//! the responsive rule says otherwise, leaving two sources of truth to disagree.
//! Anything that is a property of the whole app rather than of one pane belongs
//! in [`crate::config`]; the theme is not a per-pane choice.
//!
//! Settings are per pane rather than per kind, so two visualizers can differ: a
//! dense one in a wide pane and a coarse one in a strip is the point of a layout
//! that lets you have both.
//!
//! [`PaneSettings`] is keyed by kind *within* one pane, so flipping the picker to
//! look at another kind and coming back finds the pane as it was left. Discarding
//! on change would be simpler and would match how [`crate::pane::PaneStates`]
//! resets runtime state, but runtime state is reconstructible and tuning is not:
//! a misclick in the picker would silently throw away work done by hand.
//!
//! The map holds only kinds the user actually touched. A pane left at its
//! defaults stores nothing, so a layout file nobody has customized is unchanged
//! from one written before this existed, and the common entry stays small enough
//! that per-pane settings cost nothing to carry around.
//!
//! Every field is an enum rather than a number. The values that matter are few
//! and each has a reason to exist, so a picker offering three named choices is
//! both easier to present than a slider and impossible to set to something
//! nobody tested. `Display` is what a `pick_list` shows, and each defers to its
//! own `label`, so the words exist in one place rather than drifting between a
//! picker and a tooltip.
//!
//! [`Density`] names a target pitch in pixels rather than a bar count, so the
//! count still falls out of the pane's width: a denser setting means more bars
//! in the same space rather than a fixed number that stops fitting as the pane
//! narrows. [`Caps`] is deliberately independent of the global rounded-corners
//! preference — that one is about panels, whose corners are large enough for a
//! single radius to suit them all, where a bar a few pixels wide needs its own
//! answer or ends up square or absurdly domed.
//!
//! [`Settings::as_visualizer`] returns an `Option` that is infallible while
//! there is one variant, which is why clippy wants it gone. It stays because the
//! map is keyed by kind and asking for the wrong one is exactly what the `None`
//! is for; adding a second variant should not also change that signature and
//! every call site with it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::PaneKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Settings {
    Visualizer(Visualizer),
}

impl Settings {
    pub fn default_for(kind: PaneKind) -> Option<Self> {
        match kind {
            PaneKind::Visualizer => Some(Self::Visualizer(Visualizer::default())),
            _ => None,
        }
    }

    pub fn kind(self) -> PaneKind {
        match self {
            Self::Visualizer(_) => PaneKind::Visualizer,
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    pub fn as_visualizer(self) -> Option<Visualizer> {
        match self {
            Self::Visualizer(settings) => Some(settings),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PaneSettings {
    by_kind: BTreeMap<PaneKind, Settings>,
}

impl PaneSettings {
    pub fn is_empty(&self) -> bool {
        self.by_kind.is_empty()
    }

    pub fn get(&self, kind: PaneKind) -> Option<Settings> {
        self.by_kind
            .get(&kind)
            .copied()
            .or_else(|| Settings::default_for(kind))
    }

    pub fn visualizer(&self) -> Visualizer {
        self.get(PaneKind::Visualizer)
            .and_then(Settings::as_visualizer)
            .unwrap_or_default()
    }

    pub fn set(&mut self, settings: Settings) {
        self.by_kind.insert(settings.kind(), settings);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    Fine,
    #[default]
    Normal,
    Coarse,
}

impl Density {
    pub const ALL: [Self; 3] = [Self::Fine, Self::Normal, Self::Coarse];

    pub fn pitch(self) -> f32 {
        match self {
            Self::Fine => 7.0,
            Self::Normal => 12.0,
            Self::Coarse => 20.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Fine => "Fine",
            Self::Normal => "Normal",
            Self::Coarse => "Coarse",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Caps {
    Square,
    #[default]
    Soft,
    Round,
}

impl Caps {
    pub const ALL: [Self; 3] = [Self::Square, Self::Soft, Self::Round];

    pub fn radius(self, width: f32) -> f32 {
        match self {
            Self::Square => 0.0,
            Self::Soft => (width * 0.2).min(2.0),
            Self::Round => width / 2.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Square => "Square",
            Self::Soft => "Soft",
            Self::Round => "Round",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tint {
    Flat,
    #[default]
    Amplitude,
    Spectrum,
}

impl Tint {
    pub const ALL: [Self; 3] = [Self::Flat, Self::Amplitude, Self::Spectrum];

    pub fn label(self) -> &'static str {
        match self {
            Self::Flat => "Flat",
            Self::Amplitude => "By level",
            Self::Spectrum => "By frequency",
        }
    }
}

macro_rules! display_via_label {
    ($($kind:ty),+ $(,)?) => {
        $(
            impl std::fmt::Display for $kind {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str(self.label())
                }
            }
        )+
    };
}

display_via_label!(Density, Caps, Tint);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Visualizer {
    #[serde(default)]
    pub density: Density,
    #[serde(default)]
    pub caps: Caps,
    #[serde(default)]
    pub tint: Tint,
    #[serde(default)]
    pub peak_hold: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_untouched_pane_stores_nothing() {
        assert!(
            PaneSettings::default().is_empty(),
            "defaults should not be written down"
        );
    }

    #[test]
    fn an_untouched_pane_still_answers_with_defaults() {
        assert_eq!(PaneSettings::default().visualizer(), Visualizer::default());
    }

    #[test]
    fn a_kind_with_nothing_to_configure_has_no_settings() {
        assert_eq!(Settings::default_for(PaneKind::Library), None);
        assert_eq!(Settings::default_for(PaneKind::Empty), None);
        assert!(Settings::default_for(PaneKind::Visualizer).is_some());
    }

    #[test]
    fn a_setting_is_filed_under_the_kind_it_belongs_to() {
        let mut settings = PaneSettings::default();
        settings.set(Settings::Visualizer(Visualizer {
            density: Density::Coarse,
            ..Visualizer::default()
        }));

        assert_eq!(settings.visualizer().density, Density::Coarse);
        assert_eq!(settings.get(PaneKind::Queue), None);
    }

    #[test]
    fn changing_kind_and_returning_keeps_the_tuning() {
        let mut settings = PaneSettings::default();
        settings.set(Settings::Visualizer(Visualizer {
            peak_hold: true,
            ..Visualizer::default()
        }));

        assert!(settings.visualizer().peak_hold);
    }

    #[test]
    fn a_denser_setting_asks_for_a_narrower_pitch() {
        assert!(Density::Fine.pitch() < Density::Normal.pitch());
        assert!(Density::Normal.pitch() < Density::Coarse.pitch());
    }

    #[test]
    fn every_density_asks_for_a_usable_pitch() {
        for density in Density::ALL {
            assert!(
                density.pitch() >= 1.0,
                "{density:?} would put bars inside each other"
            );
        }
    }

    #[test]
    fn square_caps_are_actually_square() {
        for width in [1.0, 6.0, 40.0] {
            assert!(Caps::Square.radius(width).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn a_cap_never_exceeds_half_the_bar() {
        for caps in Caps::ALL {
            for width in [1.0, 3.0, 12.0, 80.0] {
                assert!(
                    caps.radius(width) <= width / 2.0 + f32::EPSILON,
                    "{caps:?} at {width}px asked for more than a half-width dome"
                );
            }
        }
    }

    #[test]
    fn soft_caps_stay_subtle_on_a_wide_bar() {
        assert!(
            Caps::Soft.radius(80.0) <= 2.0,
            "a soft cap should not become a dome just because the bar is wide"
        );
        assert!(Caps::Soft.radius(80.0) < Caps::Round.radius(80.0));
    }

    #[test]
    fn every_choice_is_labelled() {
        for density in Density::ALL {
            assert!(!density.label().is_empty());
        }
        for caps in Caps::ALL {
            assert!(!caps.label().is_empty());
        }
        for tint in Tint::ALL {
            assert!(!tint.label().is_empty());
        }
    }

    #[test]
    fn the_defaults_are_what_the_pane_already_did() {
        let visualizer = Visualizer::default();
        assert!((visualizer.density.pitch() - 12.0).abs() < f32::EPSILON);
        assert_eq!(visualizer.tint, Tint::Amplitude);
        assert!(!visualizer.peak_hold);
    }

    #[test]
    fn settings_survive_a_round_trip_through_the_layout_file() {
        let mut settings = PaneSettings::default();
        settings.set(Settings::Visualizer(Visualizer {
            density: Density::Coarse,
            caps: Caps::Round,
            tint: Tint::Spectrum,
            peak_hold: true,
        }));

        let text = toml::to_string(&settings).expect("settings serialize");
        let back: PaneSettings = toml::from_str(&text).expect("settings parse");

        assert_eq!(back, settings);
    }
}
