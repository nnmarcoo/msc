pub struct StyledButton<'a, F>
where
    F: FnMut(),
{
    size: egui::Vec2,
    image: &'a egui::Image<'a>,
    hover_color: Option<egui::Color32>,
    rounding: f32,
    on_click: F,
}

impl<'a, F> StyledButton<'a, F>
where
    F: FnMut(),
{
    pub fn new(size: egui::Vec2, image: &'a egui::Image<'a>, on_click: F) -> Self {
        Self {
            size,
            image,
            hover_color: None,
            rounding: 0.,
            on_click,
        }
    }

    pub fn with_hover_color(mut self, color: egui::Color32) -> Self {
        self.hover_color = Some(color);
        self
    }

    pub fn with_rounding(mut self, rounding: f32) -> Self {
        self.rounding = rounding;
        self
    }
}

impl<'a, F> egui::Widget for StyledButton<'a, F>
where
    F: FnMut(),
{
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(self.size, egui::Sense::click());

        if response.clicked() {
            (self.on_click)();
        }

        if ui.clip_rect().intersects(rect) {
            let visuals = ui.style().interact(&response);
            let bg_color = self.hover_color.unwrap_or(visuals.bg_fill);

            if response.hovered() {
                ui.painter().rect_filled(rect, self.rounding, bg_color);
            }

            self.image.paint_at(ui, rect);
        }

        response
    }
}
