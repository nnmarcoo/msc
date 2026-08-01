//! Custom widgets that iced's built-ins cannot express.
//!
//! Anything here exists because it needs `Overlay`, drawing outside its own
//! layout bounds, or its own event handling. Plain composition belongs in the
//! view code that uses it, not in this module.

pub mod context_menu;
pub mod hover_row;
pub mod marquee;
pub mod menu;
pub mod pane_picker;
pub mod queue_list;
pub mod rating;
pub mod scrim;
pub mod search_bar;
pub mod spectrum;
pub mod theme_picker;
pub mod timeline;
pub mod tooltip;
pub mod track_list;
pub mod volume;
