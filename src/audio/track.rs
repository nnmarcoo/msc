use symphonia::core::codecs::{CodecParameters, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, Track as SymphoniaTrack};
use symphonia::core::io::{MediaSourceStream, MediaSource};
use symphonia::core::meta::{MetadataOptions, Tag, Visual};
use symphonia::core::probe::Hint;
use symphonia::core::units::{Time};
use symphonia::default::get_probe;
use std::fs::File;
use std::path::Path;

pub struct Track {
    codec: String,
    sample_rate: u32,
    channels: u16,
    duration: u64,
    metadata: Vec<Tag>,
    visuals: Vec<Visual>,
}

impl Track {
    pub fn new(file_path: &Path) -> Result<Self, symphonia::core::errors::Error> {
        // Open the file
        let file = File::open(file_path)?;
        
        // Create a media source stream
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        
        // Create a probe hint and use it to get the format
        let hint = Hint::new();
        let probe = get_probe().format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())?;
        
        // Get the format reader and audio track
        let format_reader = probe.format;
        let track = format_reader.default_track().ok_or_else(|| symphonia::core::errors::Error::Unsupported("No default track found"))?;
        
        // Extract the codec parameters
        let codec_params = track.codec_params.clone();
        
        // Collect metadata if available
        let mut metadata = Vec::new();
        if let Some(revision) = format_reader.metadata().current() {
            for tag in revision.tags() {
                metadata.push(tag.clone());
            }
        }

        // Collect visuals if available
        let mut visuals = Vec::new();
        if let Some(revision) = format_reader.metadata().current() {
            for visual in revision.visuals() {
                visuals.push(visual.clone());
            }
        }

        let dur = track.codec_params.n_frames.map(|frames| track.codec_params.start_ts + frames).unwrap();

        // Initialize and return the Track struct with the extracted information
        Ok(Self {
            codec: codec_params.codec.to_string(),
            sample_rate: codec_params.sample_rate.unwrap_or(0),
            channels: codec_params.channels.map(|c| c.count() as u16).unwrap_or(0), // Fixed channels extraction
            duration: dur,
            metadata,
            visuals,
        })
    }
}
