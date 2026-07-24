use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::sync::Arc;

pub const NUM_BINS: usize = 32;

const FFT_SIZE: usize = 2048;
const MIN_FREQ: f32 = 60.0;
const MAX_FREQ: f32 = 16_000.0;

/// Bin smoothing: rises quickly, falls slowly, so peaks stay readable.
const ATTACK: f32 = 0.4;
const FALL: f32 = 0.80;
/// Applied to every bin while the signal is below the silence threshold.
const DECAY: f32 = 0.92;

/// Automatic gain control. Tracks the loudest bin so quiet passages still fill
/// the display, rising fast and falling slowly to avoid pumping.
const AGC_ATTACK: f32 = 0.3;
const AGC_RELEASE: f32 = 0.995;
const RANGE_DB: f32 = 50.0;
const SILENCE_THRESHOLD: f32 = -70.0;

const DEFAULT_SAMPLE_RATE: f32 = 44_100.0;

/// Log-spaced FFT magnitude bins, normalised to 0..1 for display.
///
/// Accumulates mono samples until a full FFT block is ready, then transforms,
/// groups the result into [`NUM_BINS`] logarithmic bands, and applies AGC and
/// smoothing. The normalisation stays here on the audio thread so consumers
/// receive display-ready values.
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

    /// Recomputes the bin map when the output sample rate changes.
    pub(super) fn set_sample_rate(&mut self, sample_rate: f32) {
        if (sample_rate - self.sample_rate).abs() > 1.0 {
            self.sample_rate = sample_rate;
            self.bin_map = compute_bin_map(sample_rate);
        }
    }

    /// Accumulates one mono sample. Returns `true` when a block completed and
    /// [`Self::bins`] holds fresh values.
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

        // Mean power per band, in dB.
        let mut band_db = [0.0f32; NUM_BINS];
        let mut loudest = -120.0f32;
        for (band, &(lo, hi)) in self.bin_map.iter().enumerate() {
            let power: f32 = self.workspace[lo..=hi].iter().map(|c| c.norm_sqr()).sum();
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

/// Maps each display band to an inclusive range of FFT bins, spaced
/// logarithmically so low frequencies are not crushed into one band.
fn compute_bin_map(sample_rate: f32) -> [(usize, usize); NUM_BINS] {
    let hz_per_bin = sample_rate / FFT_SIZE as f32;
    let (log_min, log_max) = (MIN_FREQ.ln(), MAX_FREQ.ln());

    let mut map = [(0usize, 0usize); NUM_BINS];
    for (i, entry) in map.iter_mut().enumerate() {
        let edge = |n: usize| (log_min + (log_max - log_min) * n as f32 / NUM_BINS as f32).exp();

        // Bin 0 is DC, so the low edge starts at 1.
        let lo = ((edge(i) / hz_per_bin) as usize).max(1);
        let hi = ((edge(i + 1) / hz_per_bin) as usize)
            .min(FFT_SIZE / 2 - 1)
            .max(lo);
        *entry = (lo, hi);
    }
    map
}
