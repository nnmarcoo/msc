mod meter;
mod spectrum;

use kira::{
    Frame,
    effect::{Effect, EffectBuilder},
    info::Info,
};
use std::sync::{Arc, Mutex};
use triple_buffer::{Input, Output, triple_buffer};

use meter::Meter;
use spectrum::Spectrum;

pub use meter::Levels;
pub use spectrum::NUM_BINS;

/// A snapshot of the audio for display.
#[derive(Clone, Copy, Debug, Default)]
pub struct VisData {
    bins: [f32; NUM_BINS],
    pub levels: Levels,
}

impl VisData {
    /// Log-spaced spectrum magnitudes, normalised to 0..1.
    pub fn bins(&self) -> &[f32; NUM_BINS] {
        &self.bins
    }
}

/// Reader half of the analyzer's triple buffer.
///
/// The audio thread publishes without ever blocking. The mutex guards only the
/// reader handle — which `triple_buffer` needs `&mut` for — and is uncontended
/// in practice, since a single consumer polls it.
pub struct VisReader {
    output: Mutex<Output<VisData>>,
}

impl VisReader {
    pub fn read(&self) -> VisData {
        match self.output.lock() {
            Ok(mut output) => *output.read(),
            Err(poisoned) => *poisoned.into_inner().read(),
        }
    }
}

/// Installed on kira's main track; hands back the reader the UI polls.
pub(crate) struct AnalyzerBuilder {
    input: Input<VisData>,
}

impl AnalyzerBuilder {
    pub(crate) fn new() -> (Self, Arc<VisReader>) {
        let (input, output) = triple_buffer(&VisData::default());
        let reader = Arc::new(VisReader {
            output: Mutex::new(output),
        });
        (Self { input }, reader)
    }
}

impl EffectBuilder for AnalyzerBuilder {
    type Handle = ();

    fn build(self) -> (Box<dyn Effect>, Self::Handle) {
        (Box::new(Analyzer::new(self.input)), ())
    }
}

/// Taps the audio stream to produce visualisation data.
///
/// Metering and spectrum analysis are independent — see [`meter`] and
/// [`spectrum`]. This type only feeds them and publishes the combined result,
/// once per FFT block so that both halves describe the same span of audio and
/// the triple buffer is not written at sample rate.
struct Analyzer {
    input: Input<VisData>,
    meter: Meter,
    spectrum: Spectrum,
}

impl Analyzer {
    fn new(input: Input<VisData>) -> Self {
        Self {
            input,
            meter: Meter::default(),
            spectrum: Spectrum::new(),
        }
    }
}

impl Effect for Analyzer {
    fn process(&mut self, input: &mut [Frame], dt: f64, _info: &Info) {
        if dt > 0.0 {
            self.spectrum.set_sample_rate((1.0 / dt) as f32);
        }

        for frame in input.iter() {
            self.meter.push(*frame);

            let mono = (frame.left + frame.right) * 0.5;
            if self.spectrum.push(mono) {
                self.input.write(VisData {
                    bins: self.spectrum.bins(),
                    levels: self.meter.take(),
                });
            }
        }
    }
}
