use kira::effect::{Effect, EffectBuilder};
use kira::{Frame, info::Info};
use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::{Arc, Mutex};

const FFT_SIZE: usize = 2048;
const NUM_BINS: usize = 16;

#[derive(Clone, Debug)]
pub struct VisData {
    pub frequency_bins: Vec<f32>,
    pub peak_left: f32,
    pub peak_right: f32,
    pub rms_left: f32,
    pub rms_right: f32,
}

impl Default for VisData {
    fn default() -> Self {
        Self {
            frequency_bins: vec![0.0; NUM_BINS],
            peak_left: 0.0,
            peak_right: 0.0,
            rms_left: 0.0,
            rms_right: 0.0,
        }
    }
}

pub struct AudioAnalyzerBuilder {
    shared_data: Arc<Mutex<VisData>>,
}

impl AudioAnalyzerBuilder {
    pub fn new() -> (Self, Arc<Mutex<VisData>>) {
        let shared_data = Arc::new(Mutex::new(VisData::default()));
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
    shared_data: Arc<Mutex<VisData>>,
    buffer_left: Vec<f32>,
    buffer_right: Vec<f32>,
    fft_planner: FftPlanner<f32>,
    sample_count: usize,
}

impl AudioAnalyzer {
    fn new(shared_data: Arc<Mutex<VisData>>) -> Self {
        Self {
            shared_data,
            buffer_left: Vec::with_capacity(FFT_SIZE),
            buffer_right: Vec::with_capacity(FFT_SIZE),
            fft_planner: FftPlanner::new(),
            sample_count: 0,
        }
    }

    fn analyze_spectrum(&mut self) {
        if self.buffer_left.len() < FFT_SIZE {
            return;
        }

        let fft = self.fft_planner.plan_fft_forward(FFT_SIZE);

        // Apply Hann window to reduce spectral leakage
        let mut windowed: Vec<Complex<f32>> = self.buffer_left[..FFT_SIZE]
            .iter()
            .enumerate()
            .map(|(i, &sample)| {
                let window = 0.5
                    * (1.0
                        - ((2.0 * std::f32::consts::PI * i as f32) / (FFT_SIZE as f32 - 1.0))
                            .cos());
                Complex::new(sample * window, 0.0)
            })
            .collect();

        fft.process(&mut windowed);

        // Use logarithmic binning for more natural frequency distribution
        let mut frequency_bins = vec![0.0; NUM_BINS];

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

        // Update shared data
        if let Ok(mut data) = self.shared_data.lock() {
            data.frequency_bins = frequency_bins;
        }

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

        // Update peak and RMS values
        if let Ok(mut data) = self.shared_data.lock() {
            data.peak_left = peak_left;
            data.peak_right = peak_right;
            data.rms_left = rms_left;
            data.rms_right = rms_right;
        }

        // Perform FFT analysis when we have enough samples
        if self.buffer_left.len() >= FFT_SIZE {
            self.analyze_spectrum();
        }
    }
}
