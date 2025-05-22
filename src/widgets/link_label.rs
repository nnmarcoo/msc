use eframe::egui::{self, Color32, CursorIcon, Response, RichText, Ui};

pub fn link_label_ui(ui: &mut Ui, text: RichText, underline_color: Color32) -> Response {
    let res = ui.label(text);

    if res.hovered() {
        ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);

        let rect = res.rect;
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            (2., underline_color),
        );
    }
    res
}

pub fn link_label<'a>(text: RichText, underline_color: Color32) -> impl egui::Widget + 'a {
    move |ui: &mut Ui| link_label_ui(ui, text, underline_color)
}
