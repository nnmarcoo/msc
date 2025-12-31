use image::{DynamicImage, imageops::FilterType};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct Colors {
    pub background: [u8; 3],
    pub accent: [u8; 3],
}

pub(crate) fn extract_colors(image: &DynamicImage) -> Colors {
    let img = resize(image);
    let mut buckets: HashMap<(u8, u8, u8), usize> = HashMap::new();

    for pixel in img.pixels() {
        let [r, g, b] = pixel.0;

        if !is_good_pixel(r, g, b) {
            continue;
        }

        let key = (r / 16, g / 16, b / 16);
        *buckets.entry(key).or_insert(0) += 1;
    }

    let mut sorted: Vec<_> = buckets.into_iter().collect();
    sorted.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

    let primary = sorted.get(0).map(|(c, _)| *c).unwrap_or((10, 10, 10));
    let secondary = sorted.get(1).map(|(c, _)| *c).unwrap_or(primary);

    Colors {
        background: darken(primary),
        accent: expand_color(secondary),
    }
}

fn resize(image: &DynamicImage) -> image::RgbImage {
    image.resize_exact(64, 64, FilterType::Triangle).to_rgb8()
}

fn is_good_pixel(r: u8, g: u8, b: u8) -> bool {
    let max = r.max(g).max(b) as i16;
    let min = r.min(g).min(b) as i16;
    let saturation = max - min;

    saturation > 20 && max < 245 && min > 10
}

fn darken((r, g, b): (u8, u8, u8)) -> [u8; 3] {
    [
        (r as f32 * 16.0 * 0.55) as u8,
        (g as f32 * 16.0 * 0.55) as u8,
        (b as f32 * 16.0 * 0.55) as u8,
    ]
}

fn expand_color((r, g, b): (u8, u8, u8)) -> [u8; 3] {
    [
        (r * 16).saturating_mul(2).min(255),
        (g * 16).saturating_mul(2).min(255),
        (b * 16).saturating_mul(2).min(255),
    ]
}
