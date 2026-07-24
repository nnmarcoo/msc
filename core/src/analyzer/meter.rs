use kira::Frame;

#[derive(Clone, Copy, Debug, Default)]
pub struct Levels {
    pub peak_left: f32,
    pub peak_right: f32,
    pub rms_left: f32,
    pub rms_right: f32,
}

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
