//! Turning a file into pixels at one size.
//!
//! A pure function of a path and a bucket, with no cache state of its own, run on
//! a blocking thread by [`crate::tasks`] because reading, decoding and resampling
//! a cover costs tens of milliseconds against a 16ms frame.
//!
//! Resampling is Lanczos3 from the full-resolution image, never from a smaller
//! copy, so every size is one resample from the original rather than a chain. A
//! resident master is used when the cache holds one, which skips the read and the
//! decode; a freshly decoded one is handed back for the cache to keep. A source
//! already smaller than its bucket is returned unchanged, enlarging being a cost
//! paid to look worse. An absent picture and a failed decode are answered alike,
//! since both mean there is nothing to draw and recording that is what stops the
//! file being read again on the next frame.
//!
//! [`resample`] works in linear light rather than in the values the file stores.
//! JPEG and PNG encode brightness by a curve where a byte of 128 is about a fifth
//! of full light, not half of it, so averaging those bytes is not averaging
//! light: a black and white checkerboard reduced to one pixel comes out at 128
//! rather than the correct 188, visibly dark. Decoding to linear, filtering, and
//! re-encoding costs about 16ms more on a 1280px cover, which is affordable
//! precisely because none of it is on the frame thread. Alpha skips both
//! conversions, being a coverage fraction rather than a brightness.
//!
//! [`LINEAR`] tabulates the decode direction because its input is a byte and so
//! has only 256 possible answers, turning 27 million `powf` calls on a large
//! cover into 256. The encode direction takes floats and has no such domain, so
//! it pays for a real `powf` per channel; that asymmetry is most of the added
//! cost.
//!
//! The dominant color is taken here too, from the same master, because it wants
//! an image already in memory and costs a fraction of the resample beside it.
//! It is read from the master rather than from the scaled copy so that every
//! size of one cover names the same color; see [`crate::artwork::palette`].

use std::sync::{Arc, LazyLock};

use image::{Rgba32FImage, RgbaImage, imageops::FilterType};

use crate::artwork::{Art, ArtKey, Job, Source, palette};

#[derive(Debug, Clone)]
pub struct Decoded {
    pub art: Art,
    pub master: Option<Source>,
}

impl Decoded {
    pub fn nothing(track: i64, bucket: u32) -> Self {
        Self {
            art: Art {
                track,
                key: None,
                bucket,
                width: 0,
                height: 0,
                pixels: Vec::new(),
                source_edge: 0,
                color: None,
            },
            master: None,
        }
    }
}

pub fn decode(job: &Job, master: Option<Source>) -> Decoded {
    let (key, master, fresh) = if let Some((key, image)) = master {
        (key, image, false)
    } else {
        let Some(bytes) = verse_core::extract_artwork_bytes(&job.path) else {
            return Decoded::nothing(job.track, job.bucket);
        };
        let key = ArtKey::of(&bytes);
        match image::load_from_memory(&bytes) {
            Ok(image) => (key, Arc::new(image.into_rgba8()), true),
            Err(_) => return Decoded::nothing(job.track, job.bucket),
        }
    };

    let (width, height) = (master.width(), master.height());
    let source_edge = width.max(height);

    let scaled = if source_edge <= job.bucket {
        master.as_ref().clone()
    } else {
        let ratio = job.bucket as f32 / source_edge as f32;
        let side = |value: u32| ((value as f32 * ratio).round() as u32).max(1);
        resample(master.as_ref(), side(width), side(height))
    };

    // Only for a cover being seen for the first time. A second size of one
    // already decoded is cut from a resident master, and the cache has filed
    // that image's color since the first pass; recomputing it would be the same
    // answer for the same pixels. `None` from a re-decode therefore means "ask
    // the cache", not "this cover has no color", which is why
    // [`crate::artwork::Cache::insert`] keeps the color it already holds rather
    // than overwriting it.
    //
    // Taken from the master rather than from `scaled`, so every size of one
    // cover names the same color. Reducing a 64px thumbnail again would let a
    // small pane and a large one disagree about what an album looks like.
    let color = fresh.then(|| palette::dominant(master.as_ref())).flatten();

    Decoded {
        art: Art {
            track: job.track,
            key: Some(key),
            bucket: job.bucket,
            width: scaled.width(),
            height: scaled.height(),
            pixels: scaled.into_raw(),
            source_edge,
            color,
        },
        master: fresh.then_some((key, master)),
    }
}

static LINEAR: LazyLock<[f32; 256]> =
    LazyLock::new(|| std::array::from_fn(|value| to_linear(value as f32 / 255.0)));

fn to_linear(value: f32) -> f32 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn to_srgb(value: f32) -> f32 {
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        value.powf(1.0 / 2.4).mul_add(1.055, -0.055)
    }
}

fn byte(value: f32) -> u8 {
    (value * 255.0).round().clamp(0.0, 255.0) as u8
}

fn resample(source: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    let linear = &*LINEAR;

    let mut light = Rgba32FImage::new(source.width(), source.height());
    for (out, pixel) in light.pixels_mut().zip(source.pixels()) {
        let [r, g, b, a] = pixel.0;
        out.0 = [
            linear[r as usize],
            linear[g as usize],
            linear[b as usize],
            f32::from(a) / 255.0,
        ];
    }

    let light = image::imageops::resize(&light, width, height, FilterType::Lanczos3);

    let mut encoded = RgbaImage::new(width, height);
    for (out, pixel) in encoded.pixels_mut().zip(light.pixels()) {
        let [r, g, b, a] = pixel.0;
        out.0 = [
            byte(to_srgb(r)),
            byte(to_srgb(g)),
            byte(to_srgb(b)),
            byte(a),
        ];
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn checkerboard(side: u32) -> RgbaImage {
        RgbaImage::from_fn(side, side, |x, y| {
            if (x + y) % 2 == 0 {
                Rgba([255, 255, 255, 255])
            } else {
                Rgba([0, 0, 0, 255])
            }
        })
    }

    #[test]
    fn averaging_black_and_white_gives_the_midpoint_in_light() {
        let flattened = resample(&checkerboard(64), 1, 1).get_pixel(0, 0).0[0];

        assert!(
            (185..=191).contains(&flattened),
            "half black and half white came out at {flattened}, not the ~188 that \
             is half the light; the filter is averaging encoded values"
        );
    }

    #[test]
    fn resampling_in_gamma_space_would_be_visibly_darker() {
        let board = checkerboard(64);
        let encoded = image::imageops::resize(&board, 1, 1, FilterType::Lanczos3)
            .get_pixel(0, 0)
            .0[0];

        assert!(
            resample(&board, 1, 1).get_pixel(0, 0).0[0] > encoded + 40,
            "linear resampling matched the gamma-space result, so it is not \
             converting at all"
        );
    }

    #[test]
    fn the_transfer_functions_are_inverses() {
        for step in 0..=255u16 {
            let value = f32::from(step) / 255.0;
            let round_tripped = to_srgb(to_linear(value));
            assert!(
                (round_tripped - value).abs() < 0.001,
                "{value} survived the round trip as {round_tripped}"
            );
        }
    }

    #[test]
    fn the_table_agrees_with_the_function_it_replaces() {
        for (value, &tabulated) in LINEAR.iter().enumerate() {
            let computed = to_linear(value as f32 / 255.0);
            assert!(
                (tabulated - computed).abs() < f32::EPSILON,
                "the lookup table disagrees with to_linear at {value}"
            );
        }
    }

    #[test]
    fn the_extremes_are_preserved_exactly() {
        let black = resample(&RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 255])), 2, 2);
        let white = resample(
            &RgbaImage::from_pixel(8, 8, Rgba([255, 255, 255, 255])),
            2,
            2,
        );

        assert_eq!(black.get_pixel(0, 0).0, [0, 0, 0, 255]);
        assert_eq!(white.get_pixel(0, 0).0, [255, 255, 255, 255]);
    }

    #[test]
    fn a_flat_color_survives_the_round_trip() {
        for level in [1u8, 40, 128, 200, 254] {
            let flat = RgbaImage::from_pixel(8, 8, Rgba([level, level, level, 255]));
            let out = resample(&flat, 4, 4).get_pixel(0, 0).0[0];

            assert!(
                out.abs_diff(level) <= 1,
                "a flat {level} came back as {out}, so the conversion is lossy \
                 where it should be exact"
            );
        }
    }

    #[test]
    fn alpha_is_not_treated_as_a_brightness() {
        let half = RgbaImage::from_pixel(8, 8, Rgba([255, 255, 255, 128]));
        let out = resample(&half, 4, 4).get_pixel(0, 0).0[3];

        assert!(
            out.abs_diff(128) <= 1,
            "alpha came back as {out} rather than 128, so it went through the \
             transfer function it should have skipped"
        );
    }

    #[test]
    fn resampling_keeps_the_size_it_was_asked_for() {
        let out = resample(&checkerboard(64), 20, 10);
        assert_eq!((out.width(), out.height()), (20, 10));
    }

    #[test]
    fn a_track_with_no_readable_art_answers_with_nothing() {
        let job = Job {
            track: 7,
            path: std::path::PathBuf::from("no-such-file.mp3"),
            bucket: 256,
        };

        let decoded = decode(&job, None);

        assert_eq!(decoded.art.key, None);
        assert_eq!(decoded.art.track, 7);
        assert_eq!(decoded.art.bucket, 256);
        assert!(decoded.master.is_none());
    }
}
