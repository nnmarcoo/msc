//! Custom widgets that iced's built-ins cannot express.
//!
//! Anything here exists because it needs `Overlay`, drawing outside its own
//! layout bounds, or its own event handling. Plain composition belongs in the
//! view code that uses it, not in this module.

pub mod menu;
pub mod pane_picker;
