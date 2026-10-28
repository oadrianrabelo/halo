//! Peças do estilo Fluido: gradientes, vidro e brilho.
//! Tudo é desenhado com malhas simples de triângulos, sem shaders.

use crate::theme::mix;
use crate::widgets::{Icon, TrafficAction, paint_icon};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Mesh, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2,
    emath::easing, pos2, vec2,
};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// Preenche um retângulo arredondado com uma cor calculada por ponto.
/// A malha é um leque com anéis concêntricos, o que dá gradientes suaves e um recorte perfeito.
pub fn gradient_rrect(p: &egui::Painter, rect: Rect, radius: f32, color_at: impl Fn(Pos2) -> Color32) {
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0).max(0.0);
    let corners = [
        (pos2(rect.right() - r, rect.top() + r), -FRAC_PI_2),
        (pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (pos2(rect.left() + r, rect.bottom() - r), FRAC_PI_2),
        (pos2(rect.left() + r, rect.top() + r), PI),
    ];
    let steps = 14;
    let mut perimeter = Vec::with_capacity(4 * (steps + 1));
    for (c, start) in corners {
        for i in 0..=steps {
            let a = start + FRAC_PI_2 * i as f32 / steps as f32;
            perimeter.push(c + r * vec2(a.cos(), a.sin()));
        }
    }

    let center = rect.center();
    let rings = 9;
    let n = perimeter.len() as u32;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color_at(center));
    for k in 1..=rings {
        let f = k as f32 / rings as f32;
        for &q in &perimeter {
            let v = center + (q - center) * f;
            mesh.colored_vertex(v, color_at(v));
        }
    }
    let ring = |k: u32, i: u32| 1 + (k - 1) * n + (i % n);
    for i in 0..n {
        mesh.add_triangle(0, ring(1, i), ring(1, i + 1));
    }
    for k in 2..=rings as u32 {
        for i in 0..n {
            let (a, b, c, d) = (ring(k - 1, i), ring(k - 1, i + 1), ring(k, i), ring(k, i + 1));
            mesh.add_triangle(a, c, b);
            mesh.add_triangle(b, c, d);
        }
    }
    p.add(mesh);
}

/// Fundo vivo: cor base + duas manchas de luz que flutuam devagar + brilho no topo.
pub struct Backdrop {
    pub base: Color32,
    pub glow_a: Color32,
    pub glow_b: Color32,
    pub opacity: f32,
    pub dark: bool,
    /// Segundos desde o início, usado para o movimento das manchas.
    pub time: f32,
}

impl Backdrop {
    pub fn paint(&self, p: &egui::Painter, rect: Rect, radius: f32) {
        let t = self.time * 0.07;
        let at = |fx: f32, fy: f32| pos2(rect.left() + rect.width() * fx, rect.top() + rect.height() * fy);
        let a = at(0.2 + 0.15 * t.sin(), 0.15 + 0.12 * (1.3 * t).cos());
        let b = at(0.85 + 0.1 * (0.8 * t).cos(), 0.9 + 0.1 * (1.1 * t).sin());
        let reach = rect.size().max_elem() * 0.95;
        let (strength_a, strength_b) = if self.dark { (0.62, 0.5) } else { (0.75, 0.65) };
        let alpha = (self.opacity * 255.0) as u8;

        gradient_rrect(p, rect, radius, |q| {
            let falloff = |c: Pos2| {
                let d = (q.distance(c) / reach).clamp(0.0, 1.0);
                let x = 1.0 - d;
                x * x * (3.0 - 2.0 * x) // smoothstep
            };
            let mut c = self.base;
            c = mix(c, self.glow_b, falloff(b) * strength_b);
            c = mix(c, self.glow_a, falloff(a) * strength_a);
            // Brilho sutil no topo, como uma superfície de vidro.
            let sheen = (1.0 - (q.y - rect.top()) / rect.height()).powi(3) * if self.dark { 0.07 } else { 0.25 };
            c = mix(c, Color32::WHITE, sheen);
            Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
        });

        // Borda: brilho em cima, sombra embaixo. Também esconde o serrilhado da malha.
        let cr = CornerRadius::same(radius.round().min(255.0) as u8);
        let rim = if self.dark { Color32::from_white_alpha(28) } else { Color32::from_white_alpha(140) };
        p.rect_stroke(rect.shrink(0.5), cr, Stroke::new(1.0, rim), StrokeKind::Inside);
        let outline = if self.dark { Color32::from_black_alpha(90) } else { Color32::from_black_alpha(22) };
        p.rect_stroke(rect, cr, Stroke::new(1.0, outline), StrokeKind::Inside);
    }
}

/// Anel com brilho em volta e uma "pérola" na ponta do progresso.
pub fn glow_ring(p: &egui::Painter, center: Pos2, radius: f32, width: f32, progress: f32, track: Color32, color: Color32) {
    p.circle_stroke(center, radius, Stroke::new(width, track));
    let progress = progress.clamp(0.0, 1.0);
    if progress <= 0.0 {
        return;
    }
    let start = -FRAC_PI_2;
    let n = ((progress * 120.0).ceil() as usize).max(2);
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let a = start + TAU * progress * i as f32 / n as f32;
            center + radius * vec2(a.cos(), a.sin())
        })
        .collect();
    // Camadas largas e transparentes por baixo fazem o brilho.
    for (extra, alpha) in [(4.0, 0.08), (2.2, 0.16), (1.0, 0.35)] {
        let w = width + width * extra;
        p.add(egui::Shape::line(pts.clone(), Stroke::new(w, color.gamma_multiply(alpha))));
    }
    p.circle_filled(pts[0], width / 2.0, color);
    p.add(egui::Shape::line(pts.clone(), Stroke::new(width, color)));
    let end = pts[n];
    p.circle_filled(end, width * 1.6, color.gamma_multiply(0.35));
    p.circle_filled(end, width * 0.95, Color32::WHITE);
}

/// Botão redondo de vidro.
pub fn glass_button(ui: &mut Ui, rect: Rect, id: &str, icon: Icon, fg: Color32, dark: bool, alpha: f32) -> Response {
    let resp = ui.interact(rect, ui.make_persistent_id(id), Sense::click());
    if alpha <= 0.01 {
        return resp;
    }
    let hover = ui.ctx().animate_bool_with_time_and_easing(resp.id.with("h"), resp.hovered(), 0.22, easing::back_out);
    let press = if resp.is_pointer_button_down_on() { 0.9 } else { 1.0 };
    let r = rect.width() * 0.5 * (1.0 + 0.08 * hover) * press;
    let p = ui.painter();
    let fill = if dark { Color32::from_white_alpha((18.0 + 22.0 * hover) as u8) } else { Color32::from_white_alpha((120.0 + 60.0 * hover) as u8) };
    p.circle_filled(rect.center(), r, fill.gamma_multiply(alpha));
    let rim = if dark { Color32::from_white_alpha(30) } else { Color32::from_black_alpha(14) };
    p.circle_stroke(rect.center(), r - 0.5, Stroke::new(1.0, rim.gamma_multiply(alpha)));
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(r * 0.8));
    paint_icon(p, icon_rect, icon, fg.gamma_multiply(alpha));
    resp
}

/// Botão principal: disco claro e brilhante, com halo na cor da fase.
#[allow(clippy::too_many_arguments)]
pub fn hero_button(ui: &mut Ui, rect: Rect, id: &str, icon: Icon, disc: Color32, on_disc: Color32, glow: Color32, alpha: f32) -> Response {
    let resp = ui.interact(rect, ui.make_persistent_id(id), Sense::click());
    if alpha <= 0.01 {
        return resp;
    }
    let hover = ui.ctx().animate_bool_with_time_and_easing(resp.id.with("h"), resp.hovered(), 0.28, easing::back_out);
    let press = if resp.is_pointer_button_down_on() { 0.92 } else { 1.0 };
    let r = rect.width() * 0.5 * (1.0 + 0.07 * hover) * press;
    let p = ui.painter();
    for (k, a) in [(1.9, 0.07), (1.45, 0.14), (1.18, 0.25)] {
        p.circle_filled(rect.center(), r * k, glow.gamma_multiply(a * alpha * (0.7 + 0.3 * hover)));
    }
    p.circle_filled(rect.center(), r, disc.gamma_multiply(alpha));
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(r * 0.72));
    paint_icon(p, icon_rect, icon, on_disc.gamma_multiply(alpha));
    resp
}

/// Botões de janela discretos (fechar, minimizar, preferências), monocromáticos.
pub fn window_buttons(ui: &mut Ui, left_center: Pos2, d: f32, fg: Color32, dark: bool, alpha: f32) -> TrafficAction {
    if alpha <= 0.01 {
        return TrafficAction::None;
    }
    let gap = d * 0.45;
    let mut action = TrafficAction::None;
    for i in 0..3 {
        let c = pos2(left_center.x + d / 2.0 + i as f32 * (d + gap), left_center.y);
        let rect = Rect::from_center_size(c, Vec2::splat(d));
        let resp = ui.interact(rect, ui.make_persistent_id(("winbtn", i)), Sense::click());
        let hover = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
        let p = ui.painter();
        let base = if dark { Color32::from_white_alpha((20.0 + 30.0 * hover) as u8) } else { Color32::from_black_alpha((10.0 + 18.0 * hover) as u8) };
        let fill = if i == 0 && hover > 0.0 { mix(base, Color32::from_rgb(255, 90, 90), hover * 0.8) } else { base };
        p.circle_filled(c, d / 2.0, fill.gamma_multiply(alpha));
        let g = if i == 0 && hover > 0.5 { Color32::WHITE } else { fg };
        let s = Stroke::new((d * 0.09).max(1.0), g.gamma_multiply(alpha * (0.6 + 0.4 * hover)));
        let k = d * 0.18;
        match i {
            0 => {
                p.line_segment([c + vec2(-k, -k), c + vec2(k, k)], s);
                p.line_segment([c + vec2(k, -k), c + vec2(-k, k)], s);
            }
            1 => {
                p.line_segment([c + vec2(-k * 1.2, 0.0), c + vec2(k * 1.2, 0.0)], s);
            }
            _ => paint_icon(p, Rect::from_center_size(c, Vec2::splat(d * 0.45)), Icon::Settings, s.color),
        }
        if resp.clicked() {
            action = match i {
                0 => TrafficAction::Close,
                1 => TrafficAction::Minimize,
                _ => TrafficAction::Zoom,
            };
        }
    }
    action
}

/// Etiqueta em pílula com um ponto colorido (ex.: "● Foco").
#[allow(clippy::too_many_arguments)]
pub fn chip(p: &egui::Painter, pos: Pos2, align: Align2, label: &str, size: f32, dot: Color32, text: Color32, dark: bool, alpha: f32) -> Rect {
    let font = FontId::proportional(size);
    let galley = p.layout_no_wrap(label.to_owned(), font, text.gamma_multiply(alpha));
    let pad = vec2(size * 0.75, size * 0.38);
    let dot_r = size * 0.28;
    let size_all = vec2(pad.x * 2.0 + dot_r * 2.0 + size * 0.45 + galley.size().x, galley.size().y + pad.y * 2.0);
    let rect = align.anchor_size(pos, size_all);
    if alpha <= 0.01 {
        return rect;
    }
    let fill = if dark { Color32::from_white_alpha(16) } else { Color32::from_white_alpha(150) };
    let cr = CornerRadius::same((rect.height() / 2.0) as u8);
    p.rect_filled(rect, cr, fill.gamma_multiply(alpha));
    let rim = if dark { Color32::from_white_alpha(22) } else { Color32::from_black_alpha(12) };
    p.rect_stroke(rect, cr, Stroke::new(1.0, rim.gamma_multiply(alpha)), StrokeKind::Inside);
    let dot_c = pos2(rect.left() + pad.x + dot_r, rect.center().y);
    p.circle_filled(dot_c, dot_r * 2.2, dot.gamma_multiply(0.25 * alpha));
    p.circle_filled(dot_c, dot_r, dot.gamma_multiply(alpha));
    let text_pos = pos2(dot_c.x + dot_r + size * 0.45, rect.center().y - galley.size().y / 2.0);
    p.galley(text_pos, galley, text);
    rect
}
