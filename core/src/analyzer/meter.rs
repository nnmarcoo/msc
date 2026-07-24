use kira::Frame;

/// Peak and RMS levels for one publish interval.
#[derive(Clone, Copy, Debug, Default)]
pub struct Levels {
    pub peak_left: f32,
    pub peak_right: f32,
    pub rms_left: f32,
    pub rms_right: f32,
}

/// Per-sample level metering.
///
/// Peak is the largest absolute sample seen; RMS is the root mean square over
/// the same window. Both accumulate cheaply on every frame and are drained when
/// the spectrum completes a block, which keeps the two halves of a `VisData`
/// covering the same span of audio.
#[derive(Default)]
pub(super) struct Meter {
    peak_left: f32,
    peak_right: f32,
    sum_left: f32,
    sum_right: f32,
    frames: usize,
}

impl Meter {
    pub(super) fn push(&mut self, frame: Frame) {
        self.peak_left = self.peak_left.max(frame.left.abs());
        self.peak_right = self.peak_right.max(frame.right.abs());

        self.sum_left += frame.left * frame.left;
        self.sum_right += frame.right * frame.right;
        self.frames += 1;
    }

    /// Returns the levels for the accumulated window and starts a new one.
    pub(super) fn take(&mut self) -> Levels {
        let levels = if self.frames == 0 {
            Levels::default()
        } else {
            let inv = 1.0 / self.frames as f32;
            Levels {
                peak_left: self.peak_left,
                peak_right: self.peak_right,
                rms_left: (self.sum_left * inv).sqrt(),
                rms_right: (self.sum_right * inv).sqrt(),
            }
        };

        *self = Self::default();
        levels
    }
}
