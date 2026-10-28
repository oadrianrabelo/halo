use crate::fluid::{self, Backdrop};
use crate::theme::{DigitWeight, FontChoice, LayoutKind, Style, Theme};
use crate::timer::{Phase, Settings, Timer, format_mmss};
use crate::widgets::{self as w, Icon, TrafficAction};
use crate::{fonts, glass, prefs, sound};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, emath::easing, pos2,
    vec2,
};
use std::time::{Duration, Instant};

const SETTINGS_KEY: &str = "settings";
const THEME_KEY: &str = "theme";
/// Duração da rolagem dos números e da troca de cor entre fases.
const ROLL: f32 = 0.32;
const PHASE_FADE: f32 = 0.7;

pub struct App {
    pub settings: Settings,
    pub theme: Theme,
    pub timer: Timer,
    pub prefs: prefs::State,
    pub font_error: Option<String>,
    pub glass_error: Option<String>,
    started: Instant,
    /// Pulsa a borda por alguns segundos quando uma fase termina.
    flash_until: Option<Instant>,
    /// O que já foi aplicado na janela, para só mexer quando algo mudar.
    applied: Option<Applied>,
    clock: ClockAnim,
    color: ColorAnim,
}

#[derive(PartialEq)]
struct Applied {
    font: (FontChoice, DigitWeight),
    size: Vec2,
    on_top: bool,
    glass: (bool, u32, bool),
    visuals: (Style, bool, Color32, Color32),
}

/// Último texto mostrado no relógio, para animar os dígitos que mudam.
struct ClockAnim {
    shown: String,
    prev: String,
    changed_at: Instant,
}

/// Cor da fase em transição suave.
struct ColorAnim {
    phase: Phase,
    from: Color32,
    changed_at: Instant,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings: Settings = cc.storage.and_then(|s| eframe::get_value(s, SETTINGS_KEY)).unwrap_or_default();
        let theme: Theme = cc.storage.and_then(|s| eframe::get_value(s, THEME_KEY)).unwrap_or_default();
        let now = Instant::now();
        let long_ago = now.checked_sub(Duration::from_secs(10)).unwrap_or(now);
        Self {
            timer: Timer::new(&settings),
            color: ColorAnim { phase: Phase::Focus, from: theme.focus, changed_at: long_ago },
            clock: ClockAnim { shown: String::new(), prev: String::new(), changed_at: long_ago },
            settings,
            theme,
            prefs: prefs::State::default(),
            font_error: None,
            glass_error: None,
            started: now,
            flash_until: None,
            applied: None,
        }
    }

    fn on_phase_finished(&mut self, ctx: &egui::Context, now: Instant) {
        if self.settings.sound {
            sound::notify(&self.settings.sound_file);
        }
        self.flash_until = Some(now + Duration::from_secs(3));
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Informational));
    }

    fn corner_radius(&self) -> f32 {
        let size = self.theme.window_size();
        (self.theme.corner_radius * self.theme.scale).min(size.min_elem() / 2.0)
    }

    /// Aplica na janela só o que mudou desde o último quadro.
    fn sync_window(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let t = &self.theme;
        let next = Applied {
            font: (t.font.clone(), t.digit_weight),
            size: t.window_size(),
            on_top: self.settings.always_on_top,
            glass: (t.glass, self.corner_radius().round() as u32, t.is_dark()),
            visuals: (t.style, t.is_dark(), t.long_break, t.background),
        };
        let prev = self.applied.take();

        if prev.as_ref().is_none_or(|p| p.font != next.font) {
            self.font_error = fonts::apply(ctx, &next.font.0, next.font.1).err();
        }
        if prev.as_ref().is_none_or(|p| p.size != next.size) {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(next.size));
        }
        if prev.as_ref().is_none_or(|p| p.on_top != next.on_top) {
            let level = if next.on_top { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal };
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
        }
        if prev.as_ref().is_none_or(|p| p.glass != next.glass) && glass::supported() {
            let (on, radius, dark) = next.glass;
            self.glass_error = glass::set(frame, on, radius as f32, dark).err();
        }
        if prev.as_ref().is_none_or(|p| p.visuals != next.visuals) {
            prefs::apply_visuals(ctx, &self.theme);
        }
        self.applied = Some(next);
    }

    fn shortcuts(&mut self, ctx: &egui::Context, now: Instant) {
        if ctx.memory(|m| m.focused().is_some()) {
            return; // não rouba teclas enquanto edita um campo
        }
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma) {
                self.prefs.open = !self.prefs.open;
            }
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::Q) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::Space) {
                self.timer.toggle(now);
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::R) {
                self.timer.reset(&self.settings);
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::S) {
                self.timer.skip(&self.settings, now);
            }
        });
    }

    /// Atualiza as animações de relógio e cor. Retorna se alguma ainda está em andamento.
    fn update_anims(&mut self, now: Instant, time: &str) -> bool {
        if self.clock.shown != time {
            self.clock.prev = std::mem::replace(&mut self.clock.shown, time.to_owned());
            self.clock.changed_at = now;
        }
        if self.color.phase != self.timer.phase {
            self.color.from = self.shown_accent(now, self.theme.accent(self.color.phase));
            self.color.phase = self.timer.phase;
            self.color.changed_at = now;
        }
        self.theme.animations
            && ((now - self.clock.changed_at).as_secs_f32() < ROLL || (now - self.color.changed_at).as_secs_f32() < PHASE_FADE)
    }

    fn shown_accent(&self, now: Instant, target: Color32) -> Color32 {
        if !self.theme.animations {
            return target;
        }
        let t = ((now - self.color.changed_at).as_secs_f32() / PHASE_FADE).min(1.0);
        w::lerp_color(self.color.from, target, easing::cubic_in_out(t))
    }

    fn roll(&self, now: Instant) -> f32 {
        if !self.theme.animations {
            return 1.0;
        }
        easing::cubic_out(((now - self.clock.changed_at).as_secs_f32() / ROLL).min(1.0))
    }

    fn draw(&mut self, ui: &mut egui::Ui, now: Instant) {
        let ctx = ui.ctx().clone();
        let rect = ui.max_rect();
        let t = self.theme.clone();
        let s = t.scale;
        let fluid = t.style == Style::Fluid;
        let dark = t.is_dark();
        let accent = self.shown_accent(now, t.accent(self.timer.phase));
        let running = self.timer.is_running();

        let hovered = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| rect.contains(p));
        let hover = ctx.animate_bool_with_time(Id::unique("hover"), hovered, 0.18);
        let show_controls = !t.controls_on_hover || !running || hovered;
        let controls = if fluid {
            ctx.animate_bool_with_time_and_easing(Id::unique("controls"), show_controls, 0.3, easing::cubic_out)
        } else {
            ctx.animate_bool_with_time(Id::unique("controls"), show_controls, 0.2)
        };

        // Fundo: clique inicia/pausa, arrastar move, botão direito abre as Preferências.
        let bg = ui.interact(rect, Id::unique("background"), Sense::click_and_drag());
        if bg.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        } else if bg.clicked() {
            self.timer.toggle(now);
        }
        if ctx.input(|i| i.pointer.secondary_clicked()) {
            self.prefs.open = true;
        }

        let radius_f = self.corner_radius();
        let radius = CornerRadius::same(radius_f.round().min(255.0) as u8);
        let p = ui.painter().clone();
        if fluid {
            let (glow_a, glow_b) = t.fluid_glows(accent);
            let backdrop = Backdrop {
                base: t.background,
                glow_a,
                glow_b,
                opacity: t.opacity,
                dark,
                time: (now - self.started).as_secs_f32(),
            };
            backdrop.paint(&p, rect, radius_f);
        } else {
            let [r, g, b, _] = t.background.to_array();
            p.rect_filled(rect, radius, Color32::from_rgba_unmultiplied(r, g, b, (t.opacity * 255.0) as u8));
            p.rect_stroke(rect.shrink(0.5), radius, Stroke::new(1.0, t.text.gamma_multiply(0.08)), StrokeKind::Inside);
        }
        if let Some(until) = self.flash_until.filter(|u| now < *u) {
            let phase = (until - now).as_secs_f32() * std::f32::consts::TAU / 1.2;
            let a = 0.35 + 0.35 * phase.sin();
            p.rect_stroke(rect.shrink(1.5 * s), radius, Stroke::new(3.0 * s, accent.gamma_multiply(a)), StrokeKind::Inside);
        }

        let progress = self.timer.progress(&self.settings, now);
        let roll = self.roll(now);
        let (time, prev) = (self.clock.shown.clone(), self.clock.prev.clone());
        let clock = |size: f32| FontId::new(size * s, fonts::clock());
        let track = if fluid { t.text.gamma_multiply(if dark { 0.09 } else { 0.12 }) } else { t.text.gamma_multiply(0.1) };
        let lights = if t.traffic_lights || fluid { hover } else { 0.0 };
        let cycles = self.settings.cycles_before_long.max(1);
        let label = self.timer.phase.label();

        match t.layout {
            LayoutKind::Ring => {
                let c = rect.center();
                if t.show_progress {
                    if fluid {
                        fluid::glow_ring(&p, c, 70.0 * s, 4.5 * s, progress, track, accent);
                    } else {
                        w::ring(&p, c, 72.0 * s, 5.0 * s, progress, track, accent);
                    }
                }
                let top = c + vec2(0.0, -31.0 * s);
                if t.show_phase {
                    if fluid {
                        fluid::chip(&p, top, Align2::CENTER_CENTER, label, 10.5 * s, accent, t.text, dark, 1.0 - lights);
                    } else {
                        w::caps_label(&p, top, Align2::CENTER_CENTER, label, 10.0 * s, t.secondary.gamma_multiply(1.0 - lights));
                    }
                }
                let d = if fluid { 16.0 * s } else { 11.0 * s };
                let group_w = if fluid { 3.0 * d + 0.9 * d } else { 3.0 * d + 1.2 * d };
                self.window_buttons(ui, pos2(c.x - group_w / 2.0, top.y), d, lights);
                let size = if fluid { 38.0 } else { 36.0 };
                w::clock_text(&p, c + vec2(0.0, -3.0 * s), Align2::CENTER_CENTER, &time, &prev, roll, clock(size), t.text);
                self.controls(ui, c + vec2(0.0, 27.0 * s), 31.0 * s, 24.0 * s, controls, false);
                if t.show_dots {
                    let dot_color = t.secondary.gamma_multiply(0.35);
                    w::dots(&p, c + vec2(0.0, 51.0 * s), cycles, self.timer.completed, 2.5 * s, accent, dot_color);
                }
            }
            LayoutKind::Pill => {
                let left = rect.left() + 20.0 * s;
                let label_y = rect.top() + 18.0 * s;
                let label_alpha = 1.0 - lights;
                let mut x = left;
                if t.show_phase {
                    let r = if fluid {
                        fluid::chip(&p, pos2(x - 4.0 * s, label_y), Align2::LEFT_CENTER, label, 9.5 * s, accent, t.text, dark, label_alpha)
                    } else {
                        w::caps_label(&p, pos2(x, label_y), Align2::LEFT_CENTER, label, 10.0 * s, t.secondary.gamma_multiply(label_alpha))
                    };
                    x = r.right() + 8.0 * s;
                }
                if t.show_dots {
                    let dots_w = (cycles as f32 * 9.0 - 4.0) * s;
                    let off = t.secondary.gamma_multiply(0.35 * label_alpha);
                    w::dots(&p, pos2(x + dots_w / 2.0, label_y), cycles, self.timer.completed, 2.5 * s, accent.gamma_multiply(label_alpha), off);
                }
                let d = if fluid { 15.0 * s } else { 11.0 * s };
                self.window_buttons(ui, pos2(left - 4.0 * s, label_y), d, lights);
                let time_y = rect.top() + if fluid { 42.0 } else { 40.0 } * s;
                w::clock_text(&p, pos2(left, time_y), Align2::LEFT_CENTER, &time, &prev, roll, clock(30.0), t.text);
                let cy = rect.center().y + 2.0 * s;
                self.controls(ui, pos2(rect.right() - 58.0 * s, cy), 31.0 * s, 28.0 * s, controls, false);
                if t.show_progress {
                    bar(&p, rect, 20.0 * s, 8.0 * s, 3.0 * s, progress, track, accent, fluid);
                }
            }
            LayoutKind::Minimal => {
                let c = rect.center() + vec2(0.0, -s);
                w::clock_text(&p, c, Align2::CENTER_CENTER, &time, &prev, roll, clock(30.0), t.text.gamma_multiply(1.0 - hover));
                if t.show_progress {
                    bar(&p, rect, (radius_f * 0.6).max(10.0 * s), 6.0 * s, 2.0 * s, progress, track, accent, fluid);
                }
                self.controls(ui, c + vec2(-12.0 * s, 0.0), 28.0 * s, 24.0 * s, hover, true);
            }
        }
    }

    fn window_buttons(&mut self, ui: &mut egui::Ui, left_center: Pos2, d: f32, alpha: f32) {
        let action = if self.theme.style == Style::Fluid {
            fluid::window_buttons(ui, left_center, d, self.theme.text, self.theme.is_dark(), alpha)
        } else {
            w::traffic_lights(ui, left_center, d, alpha)
        };
        match action {
            TrafficAction::Close => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
            TrafficAction::Minimize => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
            TrafficAction::Zoom => self.prefs.open = !self.prefs.open,
            TrafficAction::None => {}
        }
    }

    /// Reiniciar, iniciar/pausar e pular, centrados em `c`.
    fn controls(&mut self, ui: &mut egui::Ui, c: Pos2, gap: f32, primary: f32, alpha: f32, with_settings: bool) {
        if alpha <= 0.01 {
            return;
        }
        let now = Instant::now();
        let t = &self.theme;
        let fluid = t.style == Style::Fluid;
        let dark = t.is_dark();
        let accent = self.shown_accent(now, t.accent(self.timer.phase));
        let play = if self.timer.is_running() { Icon::Pause } else { Icon::Play };
        let text = t.text;
        // No Fluido os botões "sobem" ao aparecer.
        let c = if fluid { c + vec2(0.0, 4.0 * t.scale * (1.0 - alpha)) } else { c };
        let side = Vec2::splat(primary * if fluid { 0.78 } else { 0.7 });
        let at = |dx: f32, size: Vec2| Rect::from_center_size(c + vec2(dx, 0.0), size);

        let small = |ui: &mut egui::Ui, dx: f32, id: &str, icon: Icon| {
            if fluid {
                fluid::glass_button(ui, at(dx, side), id, icon, text, dark, alpha)
            } else {
                w::icon_button(ui, at(dx, side), id, icon, text, alpha)
            }
        };
        if small(ui, -gap, "reset", Icon::Reset).clicked() {
            self.timer.reset(&self.settings);
        }
        let primary_rect = at(0.0, Vec2::splat(primary));
        let clicked = if fluid {
            let disc = if dark { Color32::WHITE } else { t.text };
            let on_disc = if dark { t.background } else { Color32::WHITE };
            fluid::hero_button(ui, primary_rect, "play", play, disc, on_disc, accent, alpha).clicked()
        } else {
            w::primary_button(ui, primary_rect, "play", play, accent, w::contrast_on(accent), alpha).clicked()
        };
        if clicked {
            self.timer.toggle(now);
        }
        if small(ui, gap, "skip", Icon::Skip).clicked() {
            self.timer.skip(&self.settings, now);
        }
        if with_settings && small(ui, 2.0 * gap, "prefs", Icon::Settings).clicked() {
            self.prefs.open = !self.prefs.open;
        }
    }
}

/// Barra de progresso fina no rodapé.
#[allow(clippy::too_many_arguments)]
fn bar(p: &egui::Painter, rect: Rect, inset_x: f32, inset_bottom: f32, h: f32, progress: f32, track: Color32, color: Color32, glow: bool) {
    let y = rect.bottom() - inset_bottom - h / 2.0;
    let full = Rect::from_min_max(pos2(rect.left() + inset_x, y - h / 2.0), pos2(rect.right() - inset_x, y + h / 2.0));
    let r = CornerRadius::same((h / 2.0).round() as u8);
    p.rect_filled(full, r, track);
    if progress <= 0.0 {
        return;
    }
    let mut done = full;
    done.set_width((full.width() * progress.clamp(0.0, 1.0)).max(h));
    if glow {
        for (k, a) in [(3.0, 0.1), (1.8, 0.22)] {
            let g = done.expand(h * (k - 1.0) / 2.0);
            p.rect_filled(g, CornerRadius::same((g.height() / 2.0) as u8), color.gamma_multiply(a));
        }
    }
    p.rect_filled(done, r, color);
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = Instant::now();

        if self.timer.tick(&self.settings, now).is_some() {
            self.on_phase_finished(&ctx, now);
        }
        self.sync_window(&ctx, frame);
        self.shortcuts(&ctx, now);
        let time = format_mmss(self.timer.remaining(now));
        let animating = self.update_anims(now, &time);
        self.draw(ui, now);

        if self.prefs.open {
            prefs::show(self, &ctx);
        }

        // Redesenha só o necessário: animações curtas, timer 1x por segundo, parado nunca.
        let flashing = self.flash_until.is_some_and(|u| now < u);
        if flashing || animating {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else if self.flash_until.take().is_some() {
            ctx.request_repaint();
        } else if self.timer.is_running() {
            let left = self.timer.remaining(now);
            let to_next_second = Duration::from_nanos(u64::from(left.subsec_nanos()));
            ctx.request_repaint_after(to_next_second.max(Duration::from_millis(50)));
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, SETTINGS_KEY, &self.settings);
        eframe::set_value(storage, THEME_KEY, &self.theme);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4] // fundo transparente para os cantos arredondados
    }
}
