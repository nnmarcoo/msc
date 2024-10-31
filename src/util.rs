use eframe::egui::{Context, CursorIcon, Pos2, ResizeDirection, ViewportCommand};

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

        if ctx.input(|i| i.pointer.primary_down()) {
            if let Some(direction) = app.resizing {
                ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
            }
        }
    }
}

pub fn check_resize_direction(ctx: &Context, pos: Pos2) -> Option<ResizeDirection> {
    let margin = 5.0;
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
