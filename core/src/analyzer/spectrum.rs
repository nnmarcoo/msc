//! The FFT bands the visualizer draws: log-spaced, AGC-normalized, one set of
//! levels per [`FFT_SIZE`] samples.
//!
//! [`NUM_BINS`] is the resolution ceiling for everything downstream, because the
//! GUI folds these bands down to the bar count a pane can show and never
//! interpolates past them. It is 64 rather than 32 because 32 put that ceiling
//! below what a wide pane asks for: the spectrum pane's density setting stopped
//! changing anything once the pane was wide enough to want 32 bars, which on a
//! normal window is most of the time.
//!
//! It is not higher than 64 because [`FFT_SIZE`] is what actually limits the low
//! end. The bands are log-spaced but the transform's own bins are linear, so at
//! 44.1kHz each is ~21.5Hz wide and the bands near [`MIN_FREQ`] are the first to
//! land on the same one. At 64 bands seven of them share a bin with a neighbor
//! and 62 of the 64 ranges are still distinct; at 128 that is 28, and the bass
//! end becomes copies of one value dressed up as detail. Widening the window
//! instead would buy resolution at the cost of latency, which a visualizer feels
//! more than it feels a coarse bass.
//!
//! Anything holding a `[f32; NUM_BINS]` writes its own `Default` rather than
//! deriving one: the standard library implements that trait for arrays only up
//! to 32, which is the length this used to be, so raising it turned a derive
//! that had always worked into an error some distance from here.

use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::sync::Arc;

pub const NUM_BINS: usize = 64;

const FFT_SIZE: usize = 2048;
const MIN_FREQ: f32 = 60.0;
const MAX_FREQ: f32 = 16_000.0;

const ATTACK: f32 = 0.4;
const FALL: f32 = 0.80;
const DECAY: f32 = 0.92;

const AGC_ATTACK: f32 = 0.3;
const AGC_RELEASE: f32 = 0.995;
const RANGE_DB: f32 = 50.0;
const SILENCE_THRESHOLD: f32 = -70.0;

const DEFAULT_SAMPLE_RATE: f32 = 44_100.0;

pub(super) struct Spectrum {
    buffer: [f32; FFT_SIZE],
    buffer_pos: usize,

    fft: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    workspace: Vec<Complex<f32>>,

    window: [f32; FFT_SIZE],
    bin_map: [(usize, usize); NUM_BINS],
    bins: [f32; NUM_BINS],

    agc_peak: f32,
    sample_rate: f32,
}

impl Spectrum {
    pub(super) fn new() -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let scratch_len = fft.get_inplace_scratch_len();

        Self {
            buffer: [0.0; FFT_SIZE],
            buffer_pos: 0,
            fft,
            scratch: vec![Complex::default(); scratch_len],
            workspace: vec![Complex::default(); FFT_SIZE],
            window: hann_window(),
            bin_map: compute_bin_map(DEFAULT_SAMPLE_RATE),
            bins: [0.0; NUM_BINS],
            agc_peak: -60.0,
            sample_rate: DEFAULT_SAMPLE_RATE,
        }
    }

    pub(super) fn set_sample_rate(&mut self, sample_rate: f32) {
        if (sample_rate - self.sample_rate).abs() > 1.0 {
            self.sample_rate = sample_rate;
            self.bin_map = compute_bin_map(sample_rate);
        }
    }

    pub(super) fn push(&mut self, sample: f32) -> bool {
        self.buffer[self.buffer_pos] = sample;
        self.buffer_pos += 1;

        if self.buffer_pos < FFT_SIZE {
            return false;
        }

        self.buffer_pos = 0;
        self.analyze();
        true
    }

    pub(super) fn bins(&self) -> [f32; NUM_BINS] {
        self.bins
    }

    fn analyze(&mut self) {
        for (i, &sample) in self.buffer.iter().enumerate() {
            self.workspace[i] = Complex::new(sample * self.window[i], 0.0);
        }
        self.fft
            .process_with_scratch(&mut self.workspace, &mut self.scratch);

        let mut band_db = [0.0f32; NUM_BINS];
        let mut loudest = -120.0f32;
        for (band, &(lo, hi)) in self.bin_map.iter().enumerate() {
            let power: f32 = self.workspace[lo..=hi]
                .iter()
                .map(rustfft::num_complex::Complex::norm_sqr)
                .sum();
            let db = 10.0 * (power / (hi - lo + 1) as f32 + 1e-12).log10();
            band_db[band] = db;
            loudest = loudest.max(db);
        }

        if loudest < SILENCE_THRESHOLD {
            for bin in &mut self.bins {
                *bin *= DECAY;
            }
            return;
        }

        let rate = if loudest > self.agc_peak {
            AGC_ATTACK
        } else {
            1.0 - AGC_RELEASE
        };
        self.agc_peak += (loudest - self.agc_peak) * rate;

        let floor = self.agc_peak - RANGE_DB;
        for (bin, &db) in self.bins.iter_mut().zip(band_db.iter()) {
            let target = ((db - floor) / RANGE_DB).clamp(0.0, 1.0);
            let smoothing = if target > *bin { ATTACK } else { FALL };
            *bin = *bin * smoothing + target * (1.0 - smoothing);
        }
    }
}

fn hann_window() -> [f32; FFT_SIZE] {
    use std::f32::consts::PI;

    let mut window = [0.0f32; FFT_SIZE];
    for (i, w) in window.iter_mut().enumerate() {
        *w = 0.5 - 0.5 * (2.0 * PI * i as f32 / FFT_SIZE as f32).cos();
    }
    window
}

fn compute_bin_map(sample_rate: f32) -> [(usize, usize); NUM_BINS] {
    let hz_per_bin = sample_rate / FFT_SIZE as f32;
    let (log_min, log_max) = (MIN_FREQ.ln(), MAX_FREQ.ln());

    let mut map = [(0usize, 0usize); NUM_BINS];
    for (i, entry) in map.iter_mut().enumerate() {
        let edge = |n: usize| (log_min + (log_max - log_min) * n as f32 / NUM_BINS as f32).exp();

        let lo = ((edge(i) / hz_per_bin) as usize).max(1);
        let hi = ((edge(i + 1) / hz_per_bin) as usize)
            .min(FFT_SIZE / 2 - 1)
            .max(lo);
        *entry = (lo, hi);
    }
    map
}
