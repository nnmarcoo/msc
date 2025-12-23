use crossbeam::atomic::AtomicCell;
use kira::effect::{Effect, EffectBuilder};
use kira::{Frame, info::Info};
use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::Arc;

// all ai

const FFT_SIZE: usize = 2048;
const NUM_BINS: usize = 16;

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

pub struct AudioAnalyzerBuilder {
    shared_data: Arc<AtomicCell<VisData>>,
}

impl AudioAnalyzerBuilder {
    pub fn new() -> (Self, Arc<AtomicCell<VisData>>) {
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
    buffer_left: Vec<f32>,
    buffer_right: Vec<f32>,
    fft_planner: FftPlanner<f32>,
    sample_count: usize,
    smoothed_bins: [f32; NUM_BINS],
    // Temporary storage to avoid extra load/store
    pending_peak_left: f32,
    pending_peak_right: f32,
    pending_rms_left: f32,
    pending_rms_right: f32,
    // Pre-computed Hann window coefficients
    hann_window: Vec<f32>,
}

impl AudioAnalyzer {
    fn new(shared_data: Arc<AtomicCell<VisData>>) -> Self {
        // Pre-compute Hann window coefficients once
        let hann_window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| {
                0.5 * (1.0 - ((2.0 * std::f32::consts::PI * i as f32) / (FFT_SIZE as f32 - 1.0)).cos())
            })
            .collect();

        Self {
            shared_data,
            buffer_left: Vec::with_capacity(FFT_SIZE),
            buffer_right: Vec::with_capacity(FFT_SIZE),
            fft_planner: FftPlanner::new(),
            sample_count: 0,
            smoothed_bins: [0.0; NUM_BINS],
            pending_peak_left: 0.0,
            pending_peak_right: 0.0,
            pending_rms_left: 0.0,
            pending_rms_right: 0.0,
            hann_window,
        }
    }

    fn analyze_spectrum(&mut self) {
        if self.buffer_left.len() < FFT_SIZE {
            return;
        }

        let fft = self.fft_planner.plan_fft_forward(FFT_SIZE);

        // Apply pre-computed Hann window to reduce spectral leakage
        let mut windowed: Vec<Complex<f32>> = self.buffer_left[..FFT_SIZE]
            .iter()
            .zip(&self.hann_window)
            .map(|(&sample, &window)| Complex::new(sample * window, 0.0))
            .collect();

        fft.process(&mut windowed);

        // Use logarithmic binning for more natural frequency distribution
        let mut frequency_bins = [0.0; NUM_BINS];

        // Calculate logarithmic bin edges
        let min_freq = 20.0f32; // 20 Hz
        let max_freq = (FFT_SIZE / 2) as f32; // Nyquist frequency in bins

        for (bin_idx, bin_value) in frequency_bins.iter_mut().enumerate() {
            // Logarithmic spacing
            let f_start = min_freq * (max_freq / min_freq).powf(bin_idx as f32 / NUM_BINS as f32);
            let f_end =
                min_freq * (max_freq / min_freq).powf((bin_idx + 1) as f32 / NUM_BINS as f32);

            let start = f_start.floor() as usize;
            let end = f_end.ceil().min(FFT_SIZE as f32 / 2.0) as usize;

            if start >= end || end > FFT_SIZE / 2 {
                continue;
            }

            let mut sum = 0.0;
            let mut count = 0;
            for i in start..end {
                let magnitude = windowed[i].norm();
                sum += magnitude;
                count += 1;
            }

            if count > 0 {
                let avg_magnitude = sum / count as f32;
                // Convert to dB scale and normalize
                let db = 20.0 * (avg_magnitude + 1e-10).log10();
                // Map from typical range of -60dB to 0dB to 0.0-1.0
                *bin_value = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
            }
        }

        // Apply temporal smoothing using exponential moving average
        // Use different smoothing for attack (going up) vs decay (going down)
        const ATTACK_SMOOTHING: f32 = 0.1; // Very fast attack (90% new value)
        const DECAY_SMOOTHING: f32 = 0.85; // Slow decay (85% old value)

        for (smoothed, &new_value) in self.smoothed_bins.iter_mut().zip(&frequency_bins) {
            // Use asymmetric smoothing: very fast attack, slow decay
            let new_smoothed = if new_value > *smoothed {
                // Attack: almost immediately follow increases
                *smoothed * ATTACK_SMOOTHING + new_value * (1.0 - ATTACK_SMOOTHING)
            } else {
                // Decay: slowly fall off for smooth visual
                *smoothed * DECAY_SMOOTHING + new_value * (1.0 - DECAY_SMOOTHING)
            };

            // Ensure no NaN or invalid values
            *smoothed = if new_smoothed.is_finite() {
                new_smoothed
            } else {
                new_value
            };
        }

        // Update shared data with bins AND pending peak/RMS values in one atomic store
        let data = VisData {
            bins_raw: frequency_bins,
            bins_smooth: self.smoothed_bins,
            peak_left: self.pending_peak_left,
            peak_right: self.pending_peak_right,
            rms_left: self.pending_rms_left,
            rms_right: self.pending_rms_right,
        };
        self.shared_data.store(data);

        // Clear buffers for next batch
        self.buffer_left.clear();
        self.buffer_right.clear();
    }
}

impl Effect for AudioAnalyzer {
    fn process(&mut self, input: &mut [Frame], _dt: f64, _info: &Info) {
        let mut peak_left = 0.0f32;
        let mut peak_right = 0.0f32;
        let mut sum_squares_left = 0.0f32;
        let mut sum_squares_right = 0.0f32;

        for frame in input.iter() {
            // Calculate peaks
            peak_left = peak_left.max(frame.left.abs());
            peak_right = peak_right.max(frame.right.abs());

            // Calculate RMS
            sum_squares_left += frame.left * frame.left;
            sum_squares_right += frame.right * frame.right;

            // Add to FFT buffers
            self.buffer_left.push(frame.left);
            self.buffer_right.push(frame.right);

            self.sample_count += 1;
        }

        let num_samples = input.len() as f32;
        let rms_left = (sum_squares_left / num_samples).sqrt();
        let rms_right = (sum_squares_right / num_samples).sqrt();

        // Store peak and RMS values temporarily - will be written with FFT data
        self.pending_peak_left = peak_left;
        self.pending_peak_right = peak_right;
        self.pending_rms_left = rms_left;
        self.pending_rms_right = rms_right;

        // Perform FFT analysis when we have enough samples (this will store everything)
        if self.buffer_left.len() >= FFT_SIZE {
            self.analyze_spectrum();
        } else {
            // If we're not doing FFT this frame, still update peak/RMS
            let mut data = self.shared_data.load();
            data.peak_left = peak_left;
            data.peak_right = peak_right;
            data.rms_left = rms_left;
            data.rms_right = rms_right;
            self.shared_data.store(data);
        }
    }
}
