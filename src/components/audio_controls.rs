use eframe::egui::{include_image, Context, ImageButton, TopBottomPanel};

use crate::msc::Msc;

pub fn show_audio_controls(app: &mut Msc, ctx: &Context) {
    TopBottomPanel::bottom("audio_controls")
        .exact_height(80.)
        .show(ctx, |ui| {
            let icon;
            if app.is_playing {
              icon = include_image!("../../assets/icons/play.png");
            } else {
              icon = include_image!("../../assets/icons/pause.png");
            }

            if ui
                  .add_sized(
                      [40., 40.],
                      ImageButton::new(icon).rounding(100.),
                  )
                  .clicked()
              {
                app.is_playing = !app.is_playing;
              }
        });
}
