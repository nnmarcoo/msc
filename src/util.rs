use eframe::egui::{Color32, Context, CursorIcon, Pos2, ResizeDirection, ViewportCommand};

use crate::msc::Msc;

pub fn handle_resize(app: &mut Msc, ctx: &Context) {
    if app.is_maximized || app.is_dragging {
        return;
    }

    if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
        app.resizing = check_resize_direction(ctx, pos);

        match app.resizing {
            Some(ResizeDirection::NorthWest) | Some(ResizeDirection::SouthEast) => {
                ctx.set_cursor_icon(CursorIcon::ResizeNwSe);
            }
            Some(ResizeDirection::NorthEast) | Some(ResizeDirection::SouthWest) => {
                ctx.set_cursor_icon(CursorIcon::ResizeNeSw);
            }
            Some(ResizeDirection::East) | Some(ResizeDirection::West) => {
                ctx.set_cursor_icon(CursorIcon::ResizeEast);
            }
            Some(ResizeDirection::North) | Some(ResizeDirection::South) => {
                ctx.set_cursor_icon(CursorIcon::ResizeVertical);
            }
            None => {}
        }

        if let Some(pos) = ctx.input(|i| i.pointer.press_origin()) {
            if check_resize_direction(ctx, pos) != None {
                if ctx.input(|i| i.pointer.primary_down()) {
                    if let Some(direction) = app.resizing {
                        ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
                    }
                }
            }
        }
    }
}

fn check_resize_direction(ctx: &Context, pos: Pos2) -> Option<ResizeDirection> {
    let margin = 10.;
    let window_rect = ctx.screen_rect();

    let is_left = pos.x <= window_rect.left() + margin;
    let is_right = pos.x >= window_rect.right() - margin;
    let is_top = pos.y <= window_rect.top() + margin;
    let is_bottom = pos.y >= window_rect.bottom() - margin;

    match (is_left, is_right, is_top, is_bottom) {
        (true, false, true, false) => Some(ResizeDirection::NorthWest),
        (true, false, false, true) => Some(ResizeDirection::SouthWest),
        (false, true, true, false) => Some(ResizeDirection::NorthEast),
        (false, true, false, true) => Some(ResizeDirection::SouthEast),
        (true, false, false, false) => Some(ResizeDirection::West),
        (false, true, false, false) => Some(ResizeDirection::East),
        (false, false, true, false) => Some(ResizeDirection::North),
        (false, false, false, true) => Some(ResizeDirection::South),
        _ => None,
    }
}

pub fn seconds_to_string(seconds: f32) -> String {
    let total_seconds = seconds.trunc() as u64;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    format!("{:02}:{:02}", minutes, seconds)
}

pub fn get_volume_color(value: f32) -> Color32 {
    let low_blue = Color32::from_rgb(0, 50, 80);
    let blue = Color32::from_rgb(0, 92, 128);
    let high_blue = Color32::from_rgb(0, 200, 255);

    // Clamp the value between 0 and 2
    let clamped_value = value.clamp(0.0, 2.0);

    if clamped_value <= 1.0 {
        // Interpolate between green and blue
        let t = clamped_value;
        let r = (low_blue.r() as f32 * (1.0 - t) + blue.r() as f32 * t) as u8;
        let g = (low_blue.g() as f32 * (1.0 - t) + blue.g() as f32 * t) as u8;
        let b = (low_blue.b() as f32 * (1.0 - t) + blue.b() as f32 * t) as u8;
        Color32::from_rgb(r, g, b)
    } else {
        // Interpolate between blue and red
        let t = clamped_value - 1.0;
        let r = (blue.r() as f32 * (1.0 - t) + high_blue.r() as f32 * t) as u8;
        let g = (blue.g() as f32 * (1.0 - t) + high_blue.g() as f32 * t) as u8;
        let b = (blue.b() as f32 * (1.0 - t) + high_blue.b() as f32 * t) as u8;
        Color32::from_rgb(r, g, b)
    }
}
