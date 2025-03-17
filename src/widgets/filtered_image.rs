use egui::{ColorImage, Image, Response, TextureHandle, TextureOptions, Ui, Vec2};
use image::{imageops::FilterType, load_from_memory, DynamicImage};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct FilteredImage {
    data_vec: Vec<u8>,
    size: (u32, u32),
    last_zoom: f32,
    #[serde(skip)]
    texture: Option<TextureHandle>,
}

impl FilteredImage {
    pub fn new(data: &'static [u8], size: (u32, u32)) -> Self {
        Self {
            data_vec: data.to_vec(),
            size,
            texture: None,
            last_zoom: 0.,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let base_size = Vec2::new(self.size.0 as f32, self.size.1 as f32);
        let (rect, response) = ui.allocate_exact_size(base_size, egui::Sense::hover());

        let zoom = ui.ctx().zoom_factor();

        if self.texture.is_none() || self.last_zoom != zoom {
            let scaled_width = (self.size.0 as f32 * zoom).round() as u32;
            let scaled_height = (self.size.1 as f32 * zoom).round() as u32;

            let color_image = load_image(
                load_from_memory(&self.data_vec).unwrap(),
                scaled_width,
                scaled_height,
            );
            self.texture = Some(ui.ctx().load_texture(
                "f_img",
                color_image,
                TextureOptions::NEAREST,
            ));
            self.last_zoom = zoom;
        }

        if let Some(texture) = &self.texture {
            Image::new(texture).paint_at(ui, rect);
        }
        response
    }
}

fn load_image(image: DynamicImage, width: u32, height: u32) -> ColorImage {
    let rgba_image = image
        .clone()
        .resize_exact(width, height, FilterType::Lanczos3)
        .to_rgba8();
    let dimensions = rgba_image.dimensions();
    let pixels = rgba_image.into_raw();

    ColorImage::from_rgba_unmultiplied([dimensions.0 as usize, dimensions.1 as usize], &pixels)
}
