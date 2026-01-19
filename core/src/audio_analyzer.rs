use crossbeam::atomic::AtomicCell;
use kira::effect::{Effect, EffectBuilder};
use kira::{Frame, info::Info};
use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::sync::Arc;

// all ai

const FFT_SIZE: usize = 2048;
const NUM_BINS: usize = 32;
const SAMPLE_RATE: f32 = 44100.0;

#[derive(Clone, Copy, Debug)]
pub struct VisData {
    bins_smooth: [f32; NUM_BINS],
    bins_raw: [f32; NUM_BINS],
    pub peak_left: f32,
    pub peak_right: f32,
    pub rms_left: f32,
    pub rms_right: f32,
}

impl VisData {
    pub fn bins_smooth(&self) -> &[f32] {
        &self.bins_smooth
    }

    pub fn bins_raw(&self) -> &[f32] {
        &self.bins_raw
    }
}

impl Default for VisData {
    fn default() -> Self {
        Self {
            bins_smooth: [0.0; NUM_BINS],
            bins_raw: [0.0; NUM_BINS],
            peak_left: 0.0,
            peak_right: 0.0,
            rms_left: 0.0,
            rms_right: 0.0,
        }
    }
}

pub(crate) struct AudioAnalyzerBuilder {
    shared_data: Arc<AtomicCell<VisData>>,
}

impl AudioAnalyzerBuilder {
    pub(crate) fn new() -> (Self, Arc<AtomicCell<VisData>>) {
        let shared_data = Arc::new(AtomicCell::new(VisData::default()));
        (
            Self {
                shared_data: shared_data.clone(),
            },
            shared_data,
        )
    }
}

impl EffectBuilder for AudioAnalyzerBuilder {
    type Handle = ();

    fn build(self) -> (Box<dyn Effect>, Self::Handle) {
        (Box::new(AudioAnalyzer::new(self.shared_data)), ())
    }
}

struct AudioAnalyzer {
    shared_data: Arc<AtomicCell<VisData>>,
    buffer: [f32; FFT_SIZE],
    buffer_pos: usize,
    fft: Arc<dyn Fft<f32>>,
    fft_scratch: Vec<Complex<f32>>,
    fft_buffer: Vec<Complex<f32>>,
    smoothed_bins: [f32; NUM_BINS],
    pending_peak_left: f32,
    pending_peak_right: f32,
    pending_rms_sum_left: f32,
    pending_rms_sum_right: f32,
    pending_sample_count: usize,
    hann_window: [f32; FFT_SIZE],
    bin_frequencies: [f32; NUM_BINS],
    agc_peak_db: f32,
}

impl AudioAnalyzer {
    fn new(shared_data: Arc<AtomicCell<VisData>>) -> Self {
        let mut hann_window = [0.0f32; FFT_SIZE];
        let fft_size_minus_one = (FFT_SIZE - 1) as f32;
        for (i, w) in hann_window.iter_mut().enumerate() {
            *w = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / fft_size_minus_one).cos());
        }

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let fft_scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];
        let fft_buffer = vec![Complex::new(0.0, 0.0); FFT_SIZE];

        let bin_frequencies = Self::compute_bin_frequencies();

        Self {
            shared_data,
            buffer: [0.0; FFT_SIZE],
            buffer_pos: 0,
            fft,
            fft_scratch,
            fft_buffer,
            smoothed_bins: [0.0; NUM_BINS],
            pending_peak_left: 0.0,
            pending_peak_right: 0.0,
            pending_rms_sum_left: 0.0,
            pending_rms_sum_right: 0.0,
            pending_sample_count: 0,
            hann_window,
            bin_frequencies,
            agc_peak_db: -60.0,
        }
    }

    fn compute_bin_frequencies() -> [f32; NUM_BINS] {
        let mut frequencies = [0.0f32; NUM_BINS];
        let freq_per_bin = SAMPLE_RATE / FFT_SIZE as f32;
        let min_freq = 20.0f32;
        let max_freq = SAMPLE_RATE / 2.0;
        let log_min = min_freq.ln();
        let log_max = max_freq.ln();
        let log_range = log_max - log_min;

        for (bin_idx, freq) in frequencies.iter_mut().enumerate() {
            let center_freq =
                (log_min + log_range * (bin_idx as f32 + 0.5) / NUM_BINS as f32).exp();
            *freq = center_freq / freq_per_bin;
        }

        frequencies
    }

    fn analyze_spectrum(&mut self) {
        for (i, (sample, window)) in self.buffer.iter().zip(&self.hann_window).enumerate() {
            self.fft_buffer[i] = Complex::new(sample * window, 0.0);
        }

        self.fft
            .process_with_scratch(&mut self.fft_buffer, &mut self.fft_scratch);

        let mut frequency_bins = [0.0f32; NUM_BINS];
        let mut frame_max_db = -120.0f32;

        for (bin_idx, &fft_pos) in self.bin_frequencies.iter().enumerate() {
            let idx = (fft_pos.round() as usize).clamp(0, FFT_SIZE / 2 - 1);
            let power = self.fft_buffer[idx].norm_sqr();

            let db = 10.0 * (power + 1e-10).log10();
            frequency_bins[bin_idx] = db;
            frame_max_db = frame_max_db.max(db);
        }

        // AGC: track peak with fast attack, slow release
        const AGC_ATTACK: f32 = 0.3;
        const AGC_RELEASE: f32 = 0.995;
        const MIN_RANGE_DB: f32 = 40.0;

        if frame_max_db > self.agc_peak_db {
            self.agc_peak_db = self.agc_peak_db * (1.0 - AGC_ATTACK) + frame_max_db * AGC_ATTACK;
        } else {
            self.agc_peak_db = self.agc_peak_db * AGC_RELEASE + frame_max_db * (1.0 - AGC_RELEASE);
        }

        let floor_db = self.agc_peak_db - MIN_RANGE_DB;
        let range_db = MIN_RANGE_DB;

        for bin in frequency_bins.iter_mut() {
            *bin = ((*bin - floor_db) / range_db).clamp(0.0, 1.0);
        }

        const ATTACK_SMOOTHING: f32 = 0.1;
        const DECAY_SMOOTHING: f32 = 0.85;

        for (smoothed, &new_value) in self.smoothed_bins.iter_mut().zip(&frequency_bins) {
            let new_smoothed = if new_value > *smoothed {
                *smoothed * ATTACK_SMOOTHING + new_value * (1.0 - ATTACK_SMOOTHING)
            } else {
                *smoothed * DECAY_SMOOTHING + new_value * (1.0 - DECAY_SMOOTHING)
            };

            *smoothed = if new_smoothed.is_finite() {
                new_smoothed
            } else {
                new_value
            };
        }

        let rms_left = if self.pending_sample_count > 0 {
            (self.pending_rms_sum_left / self.pending_sample_count as f32).sqrt()
        } else {
            0.0
        };
        let rms_right = if self.pending_sample_count > 0 {
            (self.pending_rms_sum_right / self.pending_sample_count as f32).sqrt()
        } else {
            0.0
        };

        let data = VisData {
            bins_raw: frequency_bins,
            bins_smooth: self.smoothed_bins,
            peak_left: self.pending_peak_left,
            peak_right: self.pending_peak_right,
            rms_left,
            rms_right,
        };
        self.shared_data.store(data);

        self.buffer_pos = 0;
        self.pending_peak_left = 0.0;
        self.pending_peak_right = 0.0;
        self.pending_rms_sum_left = 0.0;
        self.pending_rms_sum_right = 0.0;
        self.pending_sample_count = 0;
    }
}

impl Effect for AudioAnalyzer {
    fn process(&mut self, input: &mut [Frame], _dt: f64, _info: &Info) {
        if input.is_empty() {
            return;
        }

        for frame in input.iter() {
            self.pending_peak_left = self.pending_peak_left.max(frame.left.abs());
            self.pending_peak_right = self.pending_peak_right.max(frame.right.abs());

            self.pending_rms_sum_left += frame.left * frame.left;
            self.pending_rms_sum_right += frame.right * frame.right;

            let mono = (frame.left + frame.right) * 0.5;
            self.buffer[self.buffer_pos] = mono;
            self.buffer_pos += 1;

            if self.buffer_pos >= FFT_SIZE {
                self.analyze_spectrum();
            }
        }

        self.pending_sample_count += input.len();

        if self.buffer_pos < FFT_SIZE && self.buffer_pos > 0 {
            let rms_left = if self.pending_sample_count > 0 {
                (self.pending_rms_sum_left / self.pending_sample_count as f32).sqrt()
            } else {
                0.0
            };
            let rms_right = if self.pending_sample_count > 0 {
                (self.pending_rms_sum_right / self.pending_sample_count as f32).sqrt()
            } else {
                0.0
            };

            let data = VisData {
                bins_raw: self.shared_data.load().bins_raw,
                bins_smooth: self.smoothed_bins,
                peak_left: self.pending_peak_left,
                peak_right: self.pending_peak_right,
                rms_left,
                rms_right,
            };
            self.shared_data.store(data);
        }
    }
}
