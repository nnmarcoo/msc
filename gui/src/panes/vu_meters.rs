use iced::{
    Color, Element, Length, Point, Rectangle, Size, Theme, mouse,
    widget::{
        canvas::{self, Canvas, Frame, Geometry, Path, Stroke, Text},
        container,
    },
};
use verse_core::Player;

use crate::app::Message;
use crate::art_cache::ArtCache;
use crate::pane_view::{PaneView, ViewContext};
use crate::styles::PAD;

const ATTACK: f32 = 0.7;
const FALL: f32 = 0.75;
const DECAY: f32 = 0.93;

#[derive(Debug, Clone)]
pub struct VUMetersPane {
    rms_left: f32,
    rms_right: f32,
}

impl VUMetersPane {
    pub fn new() -> Self {
        Self {
            rms_left: 0.0,
            rms_right: 0.0,
        }
    }
}

impl PaneView for VUMetersPane {
    fn update(&mut self, player: &Player, _art: &mut ArtCache) {
        let viz_data = player.vis_data();
        let target_l = linear_to_pos(viz_data.rms_left);
        let target_r = linear_to_pos(viz_data.rms_right);

        if player.is_playing() {
            let coeff_l = if target_l > self.rms_left {
                ATTACK
            } else {
                FALL
            };
            let coeff_r = if target_r > self.rms_right {
                ATTACK
            } else {
                FALL
            };
            self.rms_left = self.rms_left * coeff_l + target_l * (1.0 - coeff_l);
            self.rms_right = self.rms_right * coeff_r + target_r * (1.0 - coeff_r);
        } else {
            self.rms_left *= DECAY;
            self.rms_right *= DECAY;
        }
    }

    fn view<'a>(&'a self, _ctx: ViewContext<'a>) -> Element<'a, Message> {
        container(
            Canvas::new(VUMeters {
                rms_left: self.rms_left,
                rms_right: self.rms_right,
            })
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(PAD)
        .into()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
}

struct VUMeters {
    rms_left: f32,
    rms_right: f32,
}

impl canvas::Program<Message> for VUMeters {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        _event: &canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        Some(canvas::Action::request_redraw())
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = theme.extended_palette();
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), palette.background.base.color);

        if bounds.width > bounds.height * 1.2 {
            draw_horizontal_pair(&mut frame, theme, bounds, self.rms_left, self.rms_right);
        } else {
            draw_vertical_pair(&mut frame, theme, bounds, self.rms_left, self.rms_right);
        }

        vec![frame.into_geometry()]
    }
}

fn bar_color(theme: &Theme, intensity: f32, lit: bool) -> Color {
    let c = theme.extended_palette().primary.strong.color;
    if lit {
        Color::from_rgb(
            c.r * intensity + (1.0 - intensity) * 0.2,
            c.g * intensity + (1.0 - intensity) * 0.2,
            c.b * intensity + (1.0 - intensity) * 0.2,
        )
    } else {
        Color::from_rgb(c.r * 0.2, c.g * 0.2, c.b * 0.2)
    }
}

const DB_MARKERS: &[(f32, &str)] = &[
    (0.333, "-18"),
    (0.5, "-12"),
    (0.667, "-6"),
    (0.833, "-3"),
    (1.0, "0"),
];

fn linear_to_pos(v: f32) -> f32 {
    if v <= 0.0 {
        return 0.0;
    }
    let db = 20.0 * v.log10();
    ((db + 60.0) / 60.0).clamp(0.0, 1.0)
}

fn draw_horizontal_pair(
    frame: &mut Frame,
    theme: &Theme,
    bounds: Rectangle,
    rms_l: f32,
    rms_r: f32,
) {
    let palette = theme.extended_palette();
    let label_color = palette.background.base.text.scale_alpha(0.4);
    let tick_color = palette.background.base.text.scale_alpha(0.15);

    let pad = PAD * 2.0;
    let label_w = 14.0;
    let scale_h = 18.0;

    let mx = pad + label_w;
    let mw = bounds.width - mx - pad;
    let my = pad;
    let mh = bounds.height - pad - scale_h;
    let ch_gap = 4.0;
    let ch = (mh - ch_gap) / 2.0;

    draw_h_channel(frame, theme, mx, my, mw, ch, rms_l);
    draw_h_channel(frame, theme, mx, my + ch + ch_gap, mw, ch, rms_r);

    let sz: iced::Pixels = 10.0.into();
    frame.fill_text(Text {
        content: "L".into(),
        position: Point::new(pad, my + ch * 0.5 - 5.0),
        color: label_color,
        size: sz,
        ..Text::default()
    });
    frame.fill_text(Text {
        content: "R".into(),
        position: Point::new(pad, my + ch + ch_gap + ch * 0.5 - 5.0),
        color: label_color,
        size: sz,
        ..Text::default()
    });

    let scale_y = my + mh + 4.0;
    for &(pos, label) in DB_MARKERS {
        let tx = mx + pos * mw;
        frame.fill_rectangle(Point::new(tx, scale_y), Size::new(1.0, 4.0), tick_color);
        let offset = -(label.len() as f32 * 2.8);
        frame.fill_text(Text {
            content: label.into(),
            position: Point::new(tx + offset, scale_y + 5.0),
            color: label_color,
            size: 9.0.into(),
            ..Text::default()
        });
    }
}

fn draw_h_channel(
    frame: &mut Frame,
    theme: &Theme,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    rms: f32,
) {
    let palette = theme.extended_palette();
    let n = 50usize;
    let gap = 1.5f32;
    let seg_w = ((width - gap * (n as f32 - 1.0)) / n as f32).max(1.5);
    let total_w = seg_w * n as f32 + gap * (n as f32 - 1.0);
    let x0 = x + (width - total_w) / 2.0;

    for i in 0..n {
        let seg_pos = i as f32 / (n - 1) as f32;
        let sx = x0 + i as f32 * (seg_w + gap);
        let color = bar_color(theme, seg_pos, seg_pos <= rms);
        frame.fill_rectangle(
            Point::new(sx, y + 1.0),
            Size::new(seg_w, height - 2.0),
            color,
        );
    }

    let outline = palette.background.base.text.scale_alpha(0.06);
    let border = Path::rectangle(Point::new(x0, y), Size::new(total_w, height));
    frame.stroke(
        &border,
        Stroke::default().with_color(outline).with_width(1.0),
    );
}

fn draw_vertical_pair(frame: &mut Frame, theme: &Theme, bounds: Rectangle, rms_l: f32, rms_r: f32) {
    let palette = theme.extended_palette();
    let label_color = palette.background.base.text.scale_alpha(0.4);
    let tick_color = palette.background.base.text.scale_alpha(0.15);

    let pad = PAD * 2.0;
    let label_h = 14.0;
    let scale_w = 24.0;

    let mx = pad;
    let mw = bounds.width - pad - scale_w;
    let my = pad;
    let mh = bounds.height - pad - label_h;
    let ch_gap = 4.0;
    let cw = (mw - ch_gap) / 2.0;

    draw_v_channel(frame, theme, mx, my, cw, mh, rms_l);
    draw_v_channel(frame, theme, mx + cw + ch_gap, my, cw, mh, rms_r);

    let sz: iced::Pixels = 10.0.into();
    frame.fill_text(Text {
        content: "L".into(),
        position: Point::new(mx + cw * 0.5 - 3.0, my + mh + 3.0),
        color: label_color,
        size: sz,
        ..Text::default()
    });
    frame.fill_text(Text {
        content: "R".into(),
        position: Point::new(mx + cw + ch_gap + cw * 0.5 - 3.0, my + mh + 3.0),
        color: label_color,
        size: sz,
        ..Text::default()
    });

    let scale_x = mx + mw + 4.0;
    for &(pos, label) in DB_MARKERS {
        let ty = my + mh - pos * mh;
        frame.fill_rectangle(Point::new(scale_x, ty), Size::new(4.0, 1.0), tick_color);
        frame.fill_text(Text {
            content: label.into(),
            position: Point::new(scale_x + 6.0, ty - 4.5),
            color: label_color,
            size: 9.0.into(),
            ..Text::default()
        });
    }
}

fn draw_v_channel(
    frame: &mut Frame,
    theme: &Theme,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    rms: f32,
) {
    let palette = theme.extended_palette();
    let n = 50usize;
    let gap = 1.5f32;
    let seg_h = ((height - gap * (n as f32 - 1.0)) / n as f32).max(1.5);
    let total_h = seg_h * n as f32 + gap * (n as f32 - 1.0);
    let y0 = y + (height - total_h) / 2.0;

    for i in 0..n {
        let seg_pos = i as f32 / (n - 1) as f32;
        let sy = y0 + (n - 1 - i) as f32 * (seg_h + gap);
        let color = bar_color(theme, seg_pos, seg_pos <= rms);
        frame.fill_rectangle(
            Point::new(x + 1.0, sy),
            Size::new(width - 2.0, seg_h),
            color,
        );
    }

    let outline = palette.background.base.text.scale_alpha(0.06);
    let border = Path::rectangle(Point::new(x, y0), Size::new(width, total_h));
    frame.stroke(
        &border,
        Stroke::default().with_color(outline).with_width(1.0),
    );
}
