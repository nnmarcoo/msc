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

#[derive(Clone, Copy, Debug)]
pub struct VisData {
    bins: [f32; NUM_BINS],
    pub levels: Levels,
}

impl Default for VisData {
    fn default() -> Self {
        Self {
            bins: [0.0; NUM_BINS],
            levels: Levels::default(),
        }
    }
}

impl VisData {
    pub fn bins(&self) -> &[f32; NUM_BINS] {
        &self.bins
    }
}

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
