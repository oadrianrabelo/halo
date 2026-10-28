//! Widgets com cara de macOS, desenhados à mão (sem fonte de ícones embutida).

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind,
    Ui, Vec2, pos2, text::LayoutJob, vec2,
};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy)]
pub enum Icon {
    Play,
    Pause,
    Reset,
    Skip,
    Settings,
}

pub fn paint_icon(p: &egui::Painter, r: Rect, icon: Icon, color: Color32) {
    let stroke = Stroke::new((r.width() * 0.13).max(1.4), color);
    let (c, w, h) = (r.center(), r.width(), r.height());
    match icon {
        Icon::Play => {
            // Triângulo levemente deslocado para parecer centralizado.
            let pts = vec![pos2(r.left() + w * 0.2, r.top()), pos2(r.left() + w * 0.2, r.bottom()), pos2(r.right(), c.y)];
            p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
        }
        Icon::Pause => {
            let bar = vec2(w * 0.28, h);
            p.rect_filled(Rect::from_min_size(r.left_top() + vec2(w * 0.1, 0.0), bar), w * 0.08, color);
            p.rect_filled(Rect::from_min_size(r.left_top() + vec2(w * 0.62, 0.0), bar), w * 0.08, color);
        }
        Icon::Skip => {
            let pts = vec![r.left_top(), r.left_bottom(), pos2(r.left() + w * 0.72, c.y)];
            p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
            p.rect_filled(Rect::from_min_max(pos2(r.right() - w * 0.18, r.top()), r.right_bottom()), w * 0.06, color);
        }
        Icon::Reset => {
            let rad = w * 0.44;
            let arc: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let a = -TAU * 0.22 + TAU * 0.8 * i as f32 / 24.0;
                    c + rad * vec2(a.cos(), a.sin())
                })
                .collect();
            let start = arc[0];
            p.add(Shape::line(arc, stroke));
            let tip = vec![start + vec2(-w * 0.34, -h * 0.1), start + vec2(w * 0.06, -h * 0.3), start + vec2(w * 0.06, h * 0.14)];
            p.add(Shape::convex_polygon(tip, color, Stroke::NONE));
        }
        Icon::Settings => {
            for (i, knob) in [0.3, 0.7, 0.45].into_iter().enumerate() {
                let y = r.top() + h * (0.15 + 0.35 * i as f32);
                p.line_segment([pos2(r.left(), y), pos2(r.right(), y)], Stroke::new(stroke.width * 0.75, color));
                p.circle_filled(pos2(r.left() + w * knob, y), stroke.width * 1.3, color);
            }
        }
    }
}

/// Botão de ícone sem moldura, com fundo suave ao passar o mouse.
pub fn icon_button(ui: &mut Ui, rect: Rect, id: &str, icon: Icon, color: Color32, alpha: f32) -> Response {
    let resp = ui.interact(rect, ui.make_persistent_id(id), Sense::click());
    if alpha <= 0.01 {
        return resp;
    }
    let hover = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
    let bg = color.gamma_multiply(0.14 * hover * alpha);
    ui.painter().circle_filled(rect.center(), rect.width() * 0.75, bg);
    let pressed = if resp.is_pointer_button_down_on() { 0.92 } else { 1.0 };
    let icon_rect = Rect::from_center_size(rect.center(), rect.size() * 0.62 * pressed);
    paint_icon(ui.painter(), icon_rect, icon, color.gamma_multiply(alpha));
    resp
}

/// Botão principal: círculo cheio na cor da fase.
pub fn primary_button(ui: &mut Ui, rect: Rect, id: &str, icon: Icon, accent: Color32, on_accent: Color32, alpha: f32) -> Response {
    let resp = ui.interact(rect, ui.make_persistent_id(id), Sense::click());
    if alpha <= 0.01 {
        return resp;
    }
    let hover = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
    let scale = if resp.is_pointer_button_down_on() { 0.94 } else { 1.0 + 0.04 * hover };
    let r = rect.width() * 0.5 * scale;
    ui.painter().circle_filled(rect.center(), r, accent.gamma_multiply(alpha));
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(r * 0.78));
    paint_icon(ui.painter(), icon_rect, icon, on_accent.gamma_multiply(alpha));
    resp
}

pub enum TrafficAction {
    None,
    Close,
    Minimize,
    Zoom,
}

/// Os três botões do macOS. Os símbolos aparecem ao passar o mouse sobre o grupo.
pub fn traffic_lights(ui: &mut Ui, left_center: Pos2, d: f32, alpha: f32) -> TrafficAction {
    if alpha <= 0.01 {
        return TrafficAction::None;
    }
    const COLORS: [Color32; 3] = [
        Color32::from_rgb(255, 95, 87),
        Color32::from_rgb(254, 188, 46),
        Color32::from_rgb(40, 200, 64),
    ];
    let gap = d * 0.6;
    let group = Rect::from_min_size(pos2(left_center.x, left_center.y - d / 2.0), vec2(3.0 * d + 2.0 * gap, d));
    let group_hovered = ui.rect_contains_pointer(group.expand(4.0));
    let mut action = TrafficAction::None;
    for (i, color) in COLORS.into_iter().enumerate() {
        let c = pos2(left_center.x + d / 2.0 + i as f32 * (d + gap), left_center.y);
        let rect = Rect::from_center_size(c, Vec2::splat(d));
        let resp = ui.interact(rect, ui.make_persistent_id(("traffic", i)), Sense::click());
        let fill = if resp.is_pointer_button_down_on() { color.gamma_multiply(0.75) } else { color };
        let p = ui.painter();
        p.circle_filled(c, d / 2.0, fill.gamma_multiply(alpha));
        p.circle_stroke(c, d / 2.0 - 0.25, Stroke::new(0.5, Color32::from_black_alpha((40.0 * alpha) as u8)));
        if group_hovered {
            let g = Color32::from_black_alpha((150.0 * alpha) as u8);
            let s = Stroke::new((d * 0.12).max(1.0), g);
            let k = d * 0.22;
            match i {
                0 => {
                    p.line_segment([c + vec2(-k, -k), c + vec2(k, k)], s);
                    p.line_segment([c + vec2(k, -k), c + vec2(-k, k)], s);
                }
                1 => {
                    p.line_segment([c + vec2(-k * 1.2, 0.0), c + vec2(k * 1.2, 0.0)], s);
                }
                _ => {
                    // Dois triângulos opostos, como no "zoom" do macOS.
                    let t = k * 1.1;
                    p.add(Shape::convex_polygon(vec![c + vec2(-t, -t), c + vec2(t * 0.4, -t), c + vec2(-t, t * 0.4)], g, Stroke::NONE));
                    p.add(Shape::convex_polygon(vec![c + vec2(t, t), c + vec2(-t * 0.4, t), c + vec2(t, -t * 0.4)], g, Stroke::NONE));
                }
            }
        }
        if resp.clicked() {
            action = [TrafficAction::Close, TrafficAction::Minimize, TrafficAction::Zoom]
                .into_iter()
                .nth(i)
                .unwrap_or(TrafficAction::None);
        }
    }
    action
}

/// Anel de progresso com pontas arredondadas.
pub fn ring(p: &egui::Painter, center: Pos2, radius: f32, width: f32, progress: f32, track: Color32, color: Color32) {
    p.circle_stroke(center, radius, Stroke::new(width, track));
    let progress = progress.clamp(0.0, 1.0);
    if progress <= 0.0 {
        return;
    }
    let start = -PI / 2.0;
    let n = ((progress * 96.0).ceil() as usize).max(2);
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let a = start + TAU * progress * i as f32 / n as f32;
            center + radius * vec2(a.cos(), a.sin())
        })
        .collect();
    p.circle_filled(pts[0], width / 2.0, color);
    p.circle_filled(pts[n], width / 2.0, color);
    p.add(Shape::line(pts, Stroke::new(width, color)));
}

/// Desenha o tempo com cada dígito numa célula de largura fixa, para os números não "dançarem".
/// Com `roll < 1`, os dígitos que mudaram em relação a `prev` deslizam para cima.
#[allow(clippy::too_many_arguments)]
pub fn clock_text(p: &egui::Painter, pos: Pos2, align: Align2, text: &str, prev: &str, roll: f32, font: FontId, color: Color32) {
    let (digit_w, colon_w) = p.fonts_mut(|f| {
        let digit = ('0'..='9').map(|c| f.glyph_width(&font, c)).fold(0.0, f32::max);
        (digit, f.glyph_width(&font, ':'))
    });
    let cell = |c: char| if c.is_ascii_digit() { digit_w } else { colon_w };
    let width: f32 = text.chars().map(cell).sum();
    let center = align.anchor_size(pos, vec2(width, font.size)).center();
    let animating = roll < 1.0 && prev.chars().count() == text.chars().count();
    let band = Rect::from_center_size(center, vec2(width + 8.0, font.size * 0.92));
    let clipped = p.with_clip_rect(band.intersect(p.clip_rect()));
    let shift = font.size * 0.8;
    let mut x = center.x - width / 2.0;
    let mut old = prev.chars();
    for c in text.chars() {
        let w = cell(c);
        // Os dois-pontos sobem um pouco para alinhar com o meio dos números.
        let dy = if c == ':' { -font.size * 0.06 } else { 0.0 };
        let at = |off: f32| pos2(x + w / 2.0, center.y + dy + off);
        match old.next() {
            Some(o) if animating && o != c => {
                clipped.text(at(-shift * roll), Align2::CENTER_CENTER, o, font.clone(), color.gamma_multiply(1.0 - roll));
                clipped.text(at(shift * (1.0 - roll)), Align2::CENTER_CENTER, c, font.clone(), color.gamma_multiply(roll));
            }
            _ => {
                p.text(at(0.0), Align2::CENTER_CENTER, c, font.clone(), color);
            }
        }
        x += w;
    }
}

/// Texto pequeno com espaçamento entre letras (estilo rótulo do macOS).
pub fn caps_label(p: &egui::Painter, pos: Pos2, align: Align2, text: &str, size: f32, color: Color32) -> Rect {
    let mut job = LayoutJob::default();
    job.append(
        &text.to_uppercase(),
        0.0,
        egui::TextFormat { font_id: FontId::proportional(size), color, extra_letter_spacing: size * 0.12, ..Default::default() },
    );
    let galley = p.layout_job(job);
    let rect = align.anchor_size(pos, galley.size());
    p.galley(rect.min, galley, color);
    rect
}

pub fn dots(p: &egui::Painter, center: Pos2, total: u32, filled: u32, r: f32, on: Color32, off: Color32) {
    let gap = r * 1.6;
    let width = total as f32 * 2.0 * r + (total.saturating_sub(1)) as f32 * gap;
    for i in 0..total {
        let c = pos2(center.x - width / 2.0 + r + i as f32 * (2.0 * r + gap), center.y);
        if i < filled {
            p.circle_filled(c, r, on);
        } else {
            p.circle_filled(c, r, off);
        }
    }
}

// ---------- Controles das Preferências ----------

/// Interruptor estilo iOS/macOS.
pub fn switch(ui: &mut Ui, on: &mut bool, accent: Color32) -> Response {
    let size = vec2(34.0, 20.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_with_time(resp.id, *on, 0.15);
    let off_color = ui.visuals().widgets.inactive.bg_fill;
    let bg = lerp_color(off_color, accent, t);
    ui.painter().rect_filled(rect, CornerRadius::same(10), bg);
    let knob_x = egui::lerp((rect.left() + 10.0)..=(rect.right() - 10.0), t);
    let knob = pos2(knob_x, rect.center().y);
    ui.painter().circle_filled(knob + vec2(0.0, 0.8), 8.4, Color32::from_black_alpha(40));
    ui.painter().circle_filled(knob, 8.0, Color32::WHITE);
    resp
}

/// Controle segmentado (como as abas das Preferências do macOS).
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str)], width: f32) -> Response {
    let h = 26.0;
    let (rect, mut resp) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let v = ui.visuals();
    let (track, text, dim) = (v.widgets.inactive.bg_fill, v.strong_text_color(), v.weak_text_color());
    // No escuro o segmento escolhido fica mais claro; no claro, branco.
    let raised = if v.dark_mode { lerp_color(track, text, 0.22) } else { Color32::WHITE };
    ui.painter().rect_filled(rect, CornerRadius::same(7), track);
    let seg_w = width / options.len() as f32;
    for (i, (opt, label)) in options.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + i as f32 * seg_w, rect.top()), vec2(seg_w, h)).shrink(2.0);
        let r_resp = ui.interact(r, resp.id.with(i), Sense::click());
        let selected = *value == *opt;
        if selected {
            ui.painter().rect_filled(r.translate(vec2(0.0, 0.5)), CornerRadius::same(6), Color32::from_black_alpha(30));
            ui.painter().rect_filled(r, CornerRadius::same(6), raised);
        } else if r_resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(6), raised.gamma_multiply(0.4));
        }
        let color = if selected { text } else { dim };
        ui.painter().text(r.center(), Align2::CENTER_CENTER, *label, FontId::proportional(13.0), color);
        if r_resp.clicked() && !selected {
            *value = *opt;
            resp.mark_changed();
        }
    }
    resp
}

/// Stepper: valor com botões − e +. Pensado para ficar à direita de uma linha
/// (layout da direita para a esquerda), por isso os itens entram ao contrário.
pub fn stepper(ui: &mut Ui, value: &mut u32, range: std::ops::RangeInclusive<u32>, suffix: &str) -> bool {
    ui.spacing_mut().item_spacing.x = 4.0;
    let button = |t| egui::Button::new(t).min_size(vec2(24.0, 22.0)).corner_radius(6);
    let mut changed = false;
    if ui.add_enabled(*value < *range.end(), button("+")).clicked() {
        *value += 1;
        changed = true;
    }
    let drag = egui::DragValue::new(value).range(range.clone()).suffix(suffix).speed(0.2);
    changed |= ui.add_sized([58.0, 22.0], drag).changed();
    if ui.add_enabled(*value > *range.start(), button("−")).clicked() {
        *value -= 1;
        changed = true;
    }
    changed
}

/// Um grupo de linhas com fundo arredondado, como nos Ajustes do macOS.
pub fn group(ui: &mut Ui, title: &str, add_rows: impl FnOnce(&mut Rows<'_>)) {
    if !title.is_empty() {
        ui.add_space(6.0);
        ui.label(egui::RichText::new(title).size(11.5).color(ui.visuals().weak_text_color()));
        ui.add_space(2.0);
    }
    let fill = ui.visuals().extreme_bg_color;
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(12, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let mut rows = Rows { ui, first: true };
            add_rows(&mut rows);
        });
}

pub struct Rows<'a> {
    pub ui: &'a mut Ui,
    first: bool,
}

impl Rows<'_> {
    pub fn row<R>(&mut self, label: &str, control: impl FnOnce(&mut Ui) -> R) -> R {
        if !self.first {
            let r = self.ui.available_rect_before_wrap();
            let y = r.top();
            let stroke = self.ui.visuals().widgets.noninteractive.bg_stroke;
            self.ui.painter().line_segment([pos2(r.left(), y), pos2(r.right(), y)], stroke);
        }
        self.first = false;
        self.ui
            .horizontal(|ui| {
                ui.set_min_height(32.0);
                ui.label(label);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), control).inner
            })
            .inner
    }

    /// Linha com conteúdo livre ocupando a largura toda.
    pub fn full<R>(&mut self, content: impl FnOnce(&mut Ui) -> R) -> R {
        if !self.first {
            let r = self.ui.available_rect_before_wrap();
            let stroke = self.ui.visuals().widgets.noninteractive.bg_stroke;
            self.ui.painter().line_segment([pos2(r.left(), r.top()), pos2(r.right(), r.top())], stroke);
        }
        self.first = false;
        self.ui.add_space(6.0);
        let r = content(self.ui);
        self.ui.add_space(6.0);
        r
    }
}

/// Amostra de tema: um mini pomodoro com as cores do preset.
pub fn theme_swatch(ui: &mut Ui, preset: &crate::theme::Preset, style: crate::theme::Style, selected: bool, sel_color: Color32) -> Response {
    let [bg, fg, _, accent, ..] = preset.colors.map(crate::theme::hex);
    let size = vec2(76.0, 62.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let p = ui.painter();
    let card = Rect::from_min_size(rect.min, vec2(size.x, 44.0));
    match style {
        crate::theme::Style::Classic => {
            p.rect_filled(card, CornerRadius::same(9), bg);
            p.rect_stroke(card, CornerRadius::same(9), Stroke::new(0.5, Color32::from_black_alpha(50)), StrokeKind::Inside);
            ring(p, card.center(), 13.0, 3.0, 0.65, fg.gamma_multiply(0.15), accent);
        }
        crate::theme::Style::Fluid => {
            let [a, b] = preset.glow.map(crate::theme::hex);
            let dark = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32 <= 128.0;
            crate::fluid::Backdrop { base: bg, glow_a: a, glow_b: b, opacity: 1.0, dark, time: 0.0 }.paint(p, card, 9.0);
            crate::fluid::glow_ring(p, card.center(), 12.0, 2.5, 0.65, fg.gamma_multiply(0.12), accent);
        }
    }
    if selected {
        p.rect_stroke(card.expand(2.5), CornerRadius::same(11), Stroke::new(2.0, sel_color), StrokeKind::Outside);
    } else if resp.hovered() {
        p.rect_stroke(card.expand(2.5), CornerRadius::same(11), Stroke::new(1.0, sel_color.gamma_multiply(0.4)), StrokeKind::Outside);
    }
    let text_color = if selected { ui.visuals().strong_text_color() } else { ui.visuals().text_color() };
    p.text(pos2(rect.center().x, rect.bottom() - 7.0), Align2::CENTER_CENTER, preset.name, FontId::proportional(11.0), text_color);
    resp
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

/// Cor legível sobre um fundo colorido.
pub fn contrast_on(c: Color32) -> Color32 {
    let lum = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    if lum > 170.0 { Color32::from_rgb(20, 20, 22) } else { Color32::WHITE }
}

/// Tecla desenhada (para mostrar atalhos).
pub fn keycap(ui: &mut Ui, text: &str) -> Response {
    let font = FontId::proportional(11.5);
    let color = ui.visuals().strong_text_color();
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let size = vec2(galley.size().x + 12.0, 20.0).max(vec2(20.0, 20.0));
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    let fill = ui.visuals().widgets.inactive.bg_fill;
    p.rect_filled(rect.translate(vec2(0.0, 1.0)), CornerRadius::same(5), Color32::from_black_alpha(40));
    p.rect_filled(rect, CornerRadius::same(5), fill);
    p.galley(rect.center() - galley.size() / 2.0, galley, color);
    resp
}
