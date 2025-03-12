use egui::{Context, Frame, Margin, TopBottomPanel};
pub struct TitleBar {
    pub is_dragging: bool,
    pub is_maximized: bool,
}

impl TitleBar {
    pub fn new() -> Self {
        TitleBar {
            is_dragging: false,
            is_maximized: false,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        TopBottomPanel::top("title_bar")
            .frame(Frame::default().inner_margin(Margin::ZERO))
            .show(ctx, |ui| {
                ui.label("TODO");
            });
    }
}
