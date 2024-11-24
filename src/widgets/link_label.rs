use eframe::egui::{self, Color32, CursorIcon, Response, RichText, Ui};

/// A custom label widget that becomes underlined when hovered.
///
/// ## Parameters:
/// - `text`: The text to display (can be plain text or styled with `RichText`).
/// - `underline_color`: The color of the underline when hovered.
pub fn link_label_ui(ui: &mut Ui, text: RichText, underline_color: Color32) -> Response {
    // Create a label with the provided text and get its response
    let response = ui.label(text);

    // Check if the label is hovered
    if response.hovered() {
        // Change the cursor to a pointing hand
        ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);

        // Draw an underline
        let rect = response.rect;
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            (2., underline_color), // Width and color of the underline
        );
    }

    response
}

/// Usage: Allows for idiomatic use with `ui.add(hover_underline_label(...))`.
pub fn link_label<'a>(text: RichText, underline_color: Color32) -> impl egui::Widget + 'a {
    move |ui: &mut Ui| link_label_ui(ui, text, underline_color)
}
