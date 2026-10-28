//! Ícones desenhados com formas simples, para não precisar embutir fonte de emoji.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Shape, Stroke, Vec2, pos2, vec2};
use std::f32::consts::TAU;

#[derive(Clone, Copy)]
pub enum Icon {
    Play,
    Pause,
    Reset,
    Skip,
    Settings,
    Close,
}

pub fn button(ui: &mut egui::Ui, icon: Icon, size: f32, color: Color32, tip: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let color = if resp.hovered() { color.gamma_multiply(1.4) } else { color };
    if resp.hovered() {
        ui.painter().rect_filled(rect.expand(2.0), 4.0, Color32::from_white_alpha(12));
    }
    paint(ui.painter(), rect.shrink(size * 0.18), icon, color);
    resp.on_hover_text(tip)
}

fn paint(p: &egui::Painter, r: Rect, icon: Icon, color: Color32) {
    let stroke = Stroke::new((r.width() * 0.14).max(1.5), color);
    let (c, w, h) = (r.center(), r.width(), r.height());
    match icon {
        Icon::Play => {
            let pts = vec![r.left_top() + vec2(w * 0.12, 0.0), r.left_bottom() + vec2(w * 0.12, 0.0), pos2(r.right(), c.y)];
            p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
        }
        Icon::Pause => {
            let bar = vec2(w * 0.3, h);
            p.rect_filled(Rect::from_min_size(r.left_top() + vec2(w * 0.05, 0.0), bar), 1.0, color);
            p.rect_filled(Rect::from_min_size(r.left_top() + vec2(w * 0.65, 0.0), bar), 1.0, color);
        }
        Icon::Skip => {
            let pts = vec![r.left_top(), r.left_bottom(), pos2(r.left() + w * 0.72, c.y)];
            p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
            p.rect_filled(Rect::from_min_max(pos2(r.right() - w * 0.2, r.top()), r.right_bottom()), 1.0, color);
        }
        Icon::Reset => {
            let rad = w * 0.45;
            let arc: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let a = -TAU * 0.2 + TAU * 0.8 * i as f32 / 20.0;
                    c + rad * vec2(a.cos(), a.sin())
                })
                .collect();
            let start = arc[0];
            p.add(Shape::line(arc, stroke));
            // Ponta da seta no início do arco.
            let tip = vec![start + vec2(-w * 0.32, -h * 0.12), start + vec2(w * 0.08, -h * 0.28), start + vec2(w * 0.04, h * 0.14)];
            p.add(Shape::convex_polygon(tip, color, Stroke::NONE));
        }
        Icon::Settings => {
            // Três "sliders".
            for (i, knob) in [0.3, 0.7, 0.45].into_iter().enumerate() {
                let y = r.top() + h * (0.15 + 0.35 * i as f32);
                p.line_segment([pos2(r.left(), y), pos2(r.right(), y)], Stroke::new(stroke.width * 0.7, color));
                p.circle_filled(pos2(r.left() + w * knob, y), stroke.width * 1.2, color);
            }
        }
        Icon::Close => {
            p.line_segment([r.left_top(), r.right_bottom()], stroke);
            p.line_segment([r.right_top(), r.left_bottom()], stroke);
        }
    }
}

/// Bolinhas de progresso do ciclo (cheias = focos concluídos).
pub fn dots(ui: &mut egui::Ui, total: u32, filled: u32, on: Color32, off: Color32) {
    let r = 3.0;
    let gap = 4.0;
    let (rect, _) = ui.allocate_exact_size(vec2(total as f32 * (2.0 * r + gap), 2.0 * r), Sense::hover());
    for i in 0..total {
        let center = pos2(rect.left() + r + i as f32 * (2.0 * r + gap), rect.center().y);
        if i < filled {
            ui.painter().circle_filled(center, r, on);
        } else {
            ui.painter().circle_stroke(center, r - 0.5, Stroke::new(1.0, off));
        }
    }
}
