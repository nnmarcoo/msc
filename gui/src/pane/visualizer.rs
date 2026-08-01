//! The visualizer pane: the spectrum, padded to the pane's edges.
//!
//! The pane is only the frame. [`crate::widgets::spectrum`] decides how many
//! bars a width can carry and draws them, because that choice needs the widget's
//! own laid-out bounds rather than the pane's; wrapping it in `responsive` here
//! to pass a size down would measure the same thing twice and disagree by the
//! padding.
//!
//! Nothing is drawn differently while paused. The analyzer's bins decay to
//! silence on their own, so a paused player settles to the resting line by the
//! same path a quiet passage does, and the pane needs no notion of transport
//! state to show it.
//!
//! The pane is [`crate::pane::PaneState::Stateless`]: two visualizer panes read
//! the same analyzer and there is nothing about one that should differ from the
//! other.

use iced::widget::container;
use iced::{Element, Length};

use verse_core::NUM_BINS;

use crate::app::Message;
use crate::pane::settings::Visualizer;
use crate::styles::PAD;
use crate::widgets::spectrum::Spectrum;

pub fn view<'a>(
    bins: [f32; NUM_BINS],
    settings: Visualizer,
    cover: Option<[u8; 3]>,
) -> Element<'a, Message> {
    container(Spectrum::new(bins, settings, cover))
        .padding(PAD)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
