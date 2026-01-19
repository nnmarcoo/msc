use crossbeam::atomic::AtomicCell;
use kira::effect::{Effect, EffectBuilder};
use kira::{Frame, info::Info};
use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::sync::Arc;

const FFT_SIZE: usize = 2048;
const NUM_BINS: usize = 32;
const SAMPLE_RATE: f32 = 44100.0;

const MIN_FREQ: f32 = 60.0;
const MAX_FREQ: f32 = 16000.0;

const ATTACK: f32 = 0.4;
const DECAY: f32 = 0.92;

#[derive(Clone, Copy, Debug, Default)]
pub struct VisData {
    bins: [f32; NUM_BINS],
    pub peak_left: f32,
    pub peak_right: f32,
    pub rms_left: f32,
    pub rms_right: f32,
}

impl VisData {
    pub fn bins(&self) -> &[f32] {
        &self.bins
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
    bins: [f32; NUM_BINS],
    peak_left: f32,
    peak_right: f32,
    rms_sum_left: f32,
    rms_sum_right: f32,
    sample_count: usize,
    window: [f32; FFT_SIZE],
    bin_map: [(usize, usize); NUM_BINS],
    agc_peak: f32,
}

impl AudioAnalyzer {
    fn new(shared_data: Arc<AtomicCell<VisData>>) -> Self {
        use std::f32::consts::PI;

        let mut window = [0.0f32; FFT_SIZE];
        let n = FFT_SIZE as f32;
        for (i, w) in window.iter_mut().enumerate() {
            *w = 0.5 - 0.5 * (2.0 * PI * i as f32 / n).cos();
        }

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let scratch_len = fft.get_inplace_scratch_len();

        let bin_map = Self::compute_bin_map();

        Self {
            shared_data,
            buffer: [0.0; FFT_SIZE],
            buffer_pos: 0,
            fft,
            fft_scratch: vec![Complex::default(); scratch_len],
            fft_buffer: vec![Complex::default(); FFT_SIZE],
            bins: [0.0; NUM_BINS],
            peak_left: 0.0,
            peak_right: 0.0,
            rms_sum_left: 0.0,
            rms_sum_right: 0.0,
            sample_count: 0,
            window,
            bin_map,
            agc_peak: -60.0,
        }
    }

    fn compute_bin_map() -> [(usize, usize); NUM_BINS] {
        let mut map = [(0usize, 0usize); NUM_BINS];
        let freq_per_bin = SAMPLE_RATE / FFT_SIZE as f32;
        let log_min = MIN_FREQ.ln();
        let log_max = MAX_FREQ.ln();

        for i in 0..NUM_BINS {
            let lo = (log_min + (log_max - log_min) * i as f32 / NUM_BINS as f32).exp();
            let hi = (log_min + (log_max - log_min) * (i + 1) as f32 / NUM_BINS as f32).exp();

            let lo_idx = ((lo / freq_per_bin) as usize).max(1);
            let hi_idx = ((hi / freq_per_bin) as usize)
                .min(FFT_SIZE / 2 - 1)
                .max(lo_idx);

            map[i] = (lo_idx, hi_idx);
        }
        map
    }

    fn analyze(&mut self) {
        for (i, &sample) in self.buffer.iter().enumerate() {
            self.fft_buffer[i] = Complex::new(sample * self.window[i], 0.0);
        }

        self.fft
            .process_with_scratch(&mut self.fft_buffer, &mut self.fft_scratch);

        const AGC_ATTACK: f32 = 0.3;
        const AGC_RELEASE: f32 = 0.995;
        const RANGE_DB: f32 = 50.0;
        const SILENCE_THRESHOLD: f32 = -70.0;

        let mut bin_db = [0.0f32; NUM_BINS];
        let mut frame_max = -120.0f32;

        for (i, &(lo, hi)) in self.bin_map.iter().enumerate() {
            let mut sum = 0.0f32;
            for j in lo..=hi {
                sum += self.fft_buffer[j].norm_sqr();
            }
            let avg = sum / (hi - lo + 1) as f32;
            let db = 10.0 * (avg + 1e-12).log10();
            bin_db[i] = db;
            frame_max = frame_max.max(db);
        }

        if frame_max < SILENCE_THRESHOLD {
            for bin in &mut self.bins {
                *bin *= DECAY;
            }
        } else {
            if frame_max > self.agc_peak {
                self.agc_peak += (frame_max - self.agc_peak) * AGC_ATTACK;
            } else {
                self.agc_peak += (frame_max - self.agc_peak) * (1.0 - AGC_RELEASE);
            }

            let floor = self.agc_peak - RANGE_DB;

            for (i, &db) in bin_db.iter().enumerate() {
                let normalized = ((db - floor) / RANGE_DB).clamp(0.0, 1.0);
                let smoothing = if normalized > self.bins[i] {
                    ATTACK
                } else {
                    DECAY
                };
                self.bins[i] = self.bins[i] * smoothing + normalized * (1.0 - smoothing);
            }
        }

        let inv_count = 1.0 / self.sample_count as f32;
        let rms_left = (self.rms_sum_left * inv_count).sqrt();
        let rms_right = (self.rms_sum_right * inv_count).sqrt();

        self.shared_data.store(VisData {
            bins: self.bins,
            peak_left: self.peak_left,
            peak_right: self.peak_right,
            rms_left,
            rms_right,
        });

        self.buffer_pos = 0;
        self.peak_left = 0.0;
        self.peak_right = 0.0;
        self.rms_sum_left = 0.0;
        self.rms_sum_right = 0.0;
        self.sample_count = 0;
    }
}

impl Effect for AudioAnalyzer {
    fn process(&mut self, input: &mut [Frame], _dt: f64, _info: &Info) {
        for frame in input.iter() {
            let left = frame.left.abs();
            let right = frame.right.abs();

            if left > self.peak_left {
                self.peak_left = left;
            }
            if right > self.peak_right {
                self.peak_right = right;
            }

            self.rms_sum_left += frame.left * frame.left;
            self.rms_sum_right += frame.right * frame.right;
            self.sample_count += 1;

            self.buffer[self.buffer_pos] = (frame.left + frame.right) * 0.5;
            self.buffer_pos += 1;

            if self.buffer_pos == FFT_SIZE {
                self.analyze();
            }
        }
    }
}
