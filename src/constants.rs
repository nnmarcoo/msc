use std::time::Duration;

use kira::{
    tween::{Easing, Tween},
    StartTime,
};

pub const TWEEN_INSTANT: Tween = Tween {
    start_time: StartTime::Immediate,
    duration: Duration::from_secs(0),
    easing: Easing::Linear,
};

pub const TWEEN_DEFAULT: Tween = Tween {
    start_time: StartTime::Immediate,
    duration: Duration::from_millis(100),
    easing: Easing::InOutPowi(1),
};

pub const DEFAULT_IMAGE_BYTES: &[u8] = include_bytes!("../assets/icons/default.png");
pub const DEFAULT_IMAGE_BORDER_BYTES: &[u8] = include_bytes!("../assets/icons/defaultborder.png");
pub const DEFALT_AUDIO_BYTES: &[u8] = include_bytes!("../assets/setup/placeholder.mp3");
