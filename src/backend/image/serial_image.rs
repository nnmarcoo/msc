use eframe::egui::{Color32, ColorImage};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerialImage {
    pub size: [usize; 2],
    pub pixels: Vec<u32>,
}

impl From<&ColorImage> for SerialImage {
    fn from(image: &ColorImage) -> Self {
        SerialImage {
            size: image.size,
            pixels: image
                .pixels
                .iter()
                .map(|color| {
                    let [r, g, b, a] = color.to_array();
                    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
                })
                .collect(),
        }
    }
}

impl Into<ColorImage> for SerialImage {
    fn into(self) -> ColorImage {
        let pixels = self
            .pixels
            .into_iter()
            .map(|rgba| {
                let r = ((rgba >> 24) & 0xFF) as u8;
                let g = ((rgba >> 16) & 0xFF) as u8;
                let b = ((rgba >> 8) & 0xFF) as u8;
                let a = (rgba & 0xFF) as u8;
                Color32::from_rgba_unmultiplied(r, g, b, a)
            })
            .collect();
        ColorImage {
            size: self.size,
            pixels,
        }
    }
}