use crate::fluid::{self, Backdrop};
use crate::theme::{DigitWeight, FontChoice, LayoutKind, Style, Theme};
use crate::timer::{Phase, Settings, Timer, format_mmss};
use crate::widgets::{self as w, Icon, TrafficAction};
use crate::media::{self, LyricsState, Media};
use crate::stats::{self, Stats};
use crate::render::{self, Mode};
use crate::tray::{self, Tray};
use crate::update::{self, Updater};
use crate::{fonts, glass, prefs, sound};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, emath::easing, pos2,
    vec2,
};
use std::time::{Duration, Instant};

const SETTINGS_KEY: &str = "settings";
const THEME_KEY: &str = "theme";
const STATS_KEY: &str = "stats";
/// Altura do painel de música, em escala 1.
const MUSIC_PANEL: f32 = 104.0;
/// A mais no painel com a capa e com a letra karaokê (linha seguinte).
const MUSIC_ART_EXTRA: f32 = 8.0;
const MUSIC_KARAOKE_EXTRA: f32 = 20.0;
/// Tempo da troca de cores quando muda a capa.
const ART_FADE: f32 = 1.2;
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
    pub stats: Stats,
    /// Thread de música (só existe com a função ligada).
    pub media: Option<Media>,
    /// Opções de letra com que a thread de música foi iniciada.
    media_lyrics: Option<crate::lyrics::Options>,
    /// Linha da letra atual e quando ela mudou (para animar).
    lyric: (Option<usize>, Instant),
    pub mode: Mode,
    first_frame: bool,
    tray: Option<Tray>,
    /// Já tentou criar a bandeja e o sistema não tem uma.
    tray_unavailable: bool,
    /// Janela escondida na bandeja.
    hidden: bool,
    pub updater: Updater,
    next_update_check: Instant,
    /// Reabrir o Halo ao fechar (depois de atualizar).
    relaunch: bool,
    /// Ilha aberta (mouse em cima ou fase acabando), quando o mouse saiu e quanto ela está aberta (0 a 1).
    island_open: bool,
    island_leave: Option<Instant>,
    island_t: f32,
    /// Textura da capa carregada (identificada pelo endereço do `Art`).
    art_tex: Option<(usize, egui::TextureHandle)>,
    /// Cores do gradiente em transição: de, para, quando começou e a capa de origem.
    glow: ([Color32; 2], [Color32; 2], Instant, usize),
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
    pub fn new(cc: &eframe::CreationContext<'_>, mode: Mode) -> Self {
        let mut settings: Settings = cc.storage.and_then(|s| eframe::get_value(s, SETTINGS_KEY)).unwrap_or_default();
        let theme: Theme = cc.storage.and_then(|s| eframe::get_value(s, THEME_KEY)).unwrap_or_default();
        let stats: Stats = cc.storage.and_then(|s| eframe::get_value(s, STATS_KEY)).unwrap_or_default();
        let now = Instant::now();
        let long_ago = now.checked_sub(Duration::from_secs(10)).unwrap_or(now);
        let mut prefs = prefs::State::default();
        // Primeira abertura depois de atualizar: mostra as novidades.
        if settings.last_version != update::VERSION {
            if !settings.last_version.is_empty() && update::newer(update::VERSION, &settings.last_version) {
                log::info!("Atualizado de {} para {}", settings.last_version, update::VERSION);
                prefs.open = true;
                prefs.tab = prefs::Tab::About;
                prefs.whats_new = true;
            }
            settings.last_version = update::VERSION.to_owned();
        }
        Self {
            timer: Timer::new(&settings),
            color: ColorAnim { phase: Phase::Focus, from: theme.focus, changed_at: long_ago },
            clock: ClockAnim { shown: String::new(), prev: String::new(), changed_at: long_ago },
            settings,
            theme,
            prefs,
            font_error: None,
            glass_error: None,
            started: now,
            flash_until: None,
            applied: None,
            stats,
            media: None,
            media_lyrics: None,
            lyric: (None, long_ago),
            mode,
            first_frame: true,
            tray: None,
            tray_unavailable: false,
            hidden: false,
            updater: Updater::new(cc.egui_ctx.clone()),
            // Primeira verificação alguns segundos depois de abrir, para não atrasar a inicialização.
            next_update_check: now + Duration::from_secs(5),
            relaunch: false,
            art_tex: None,
            island_open: false,
            island_leave: None,
            island_t: 0.0,
            glow: ([Color32::BLACK; 2], [Color32::BLACK; 2], long_ago, usize::MAX),
        }
    }

    fn set_hidden(&mut self, ctx: &egui::Context, hidden: bool) {
        self.hidden = hidden;
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Visible(!hidden));
        if !hidden {
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
        }
    }

    fn minimize(&mut self, ctx: &egui::Context) {
        if self.tray.is_some() && self.settings.minimize_to_tray {
            self.set_hidden(ctx, true);
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }

    /// Cria ou remove o ícone da bandeja, atualiza o desenho e trata os cliques nele.
    fn sync_tray(&mut self, ctx: &egui::Context, now: Instant) {
        if self.settings.tray && self.tray.is_none() && !self.tray_unavailable {
            self.tray = Tray::new(ctx);
            self.tray_unavailable = self.tray.is_none();
        } else if !self.settings.tray && self.tray.is_some() {
            self.tray = None;
            if self.hidden {
                self.set_hidden(ctx, false);
            }
        }
        let Some(tray) = &mut self.tray else { return };

        let running = self.timer.is_running();
        let phase = self.timer.phase.label();
        let time = format_mmss(self.timer.remaining(now));
        let tooltip = if running { format!("Halo · {phase} · {time}") } else { format!("Halo · {phase} · {time} (pausado)") };
        let [r, g, b, _] = self.theme.accent(self.timer.phase).to_array();
        let step = (self.timer.progress(&self.settings, now).clamp(0.0, 1.0) * 48.0).round() as u8;
        tray.set(tray::View { step, accent: [r, g, b], running, tooltip });

        for ev in tray.events() {
            match ev {
                tray::Event::Toggle => self.set_hidden(ctx, !self.hidden),
                tray::Event::PlayPause => self.timer.toggle(now),
                tray::Event::Skip => self.timer.skip(&self.settings, now),
                tray::Event::Preferences => {
                    self.set_hidden(ctx, false);
                    self.prefs.open = true;
                }
                tray::Event::Quit => ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close),
            }
        }
    }

    fn sync_updates(&mut self, ctx: &egui::Context, now: Instant) {
        if self.settings.auto_update {
            if now >= self.next_update_check {
                self.next_update_check = now + Duration::from_secs(6 * 3600);
                self.updater.check();
            }
            // Com o timer parado nada redesenha a tela; acorda na hora da próxima verificação.
            ctx.request_repaint_after(self.next_update_check - now);
        }
        if matches!(self.updater.state(), update::State::Ready) && !self.relaunch {
            self.relaunch = true;
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
        }
    }

    fn on_phase_finished(&mut self, ctx: &egui::Context, now: Instant, phase: Phase) {
        if phase == Phase::Focus {
            self.stats.record(stats::today(), self.settings.focus_min);
        }
        if self.settings.sound {
            let s = &self.settings;
            let which = if phase == Phase::Focus { s.sound_focus_end } else { s.sound_break_end };
            sound::play(which, s.sound_volume, &s.sound_file);
        }
        self.flash_until = Some(now + Duration::from_secs(3));
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Informational));
    }

    /// Liga, desliga ou reinicia a thread de música conforme as configurações.
    fn sync_media(&mut self, ctx: &egui::Context) {
        let want = self.settings.music;
        if !want {
            self.media = None;
        } else {
            let opts = self.settings.lyrics.then(|| crate::lyrics::Options {
                plain: self.settings.lyrics_plain,
                vagalume_key: self.settings.vagalume_key.trim().to_owned(),
            });
            if self.media.is_none() || self.media_lyrics != opts {
                self.media = Some(Media::start(ctx.clone(), opts.clone()));
                self.media_lyrics = opts;
            }
        }
    }

    fn show_music_panel(&self) -> bool {
        let Some(m) = &self.media else { return false };
        !self.settings.music_only_when_playing || m.snapshot().0.is_some()
    }

    /// Altura do painel de música (já na escala), ou 0 se ele não aparece.
    fn music_panel_height(&self) -> f32 {
        if !self.show_music_panel() {
            return 0.0;
        }
        let t = &self.theme;
        let mut panel = MUSIC_PANEL;
        if t.music_art {
            panel += MUSIC_ART_EXTRA;
        }
        if t.lyrics_style == crate::theme::LyricsStyle::Karaoke && self.settings.lyrics {
            panel += MUSIC_KARAOKE_EXTRA;
        }
        panel * t.scale
    }

    fn window_size(&self) -> Vec2 {
        if self.theme.layout == LayoutKind::Island {
            // Aberta (ou ainda fechando): tamanho do cartão; fechada: só a pílula.
            return if self.island_open || self.island_t > 0.01 {
                crate::theme::ISLAND_OPEN * self.theme.scale + vec2(0.0, self.music_panel_height())
            } else {
                self.theme.window_size()
            };
        }
        self.theme.window_size() + vec2(0.0, self.music_panel_height())
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
            size: self.window_size(),
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
        if self.theme.layout == LayoutKind::Island {
            return self.draw_island(ui, now);
        }
        let ctx = ui.ctx().clone();
        let full = ui.max_rect();
        // Área do timer; o que sobrar embaixo é o painel de música.
        let rect = Rect::from_min_size(full.min, self.theme.window_size().min(full.size()));
        let t = self.theme.clone();
        let s = t.scale;
        let fluid = t.style == Style::Fluid;
        let dark = t.is_dark();
        let accent = self.shown_accent(now, t.accent(self.timer.phase));
        let running = self.timer.is_running();

        let hovered = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| full.contains(p));
        let hover = ctx.animate_bool_with_time(Id::unique("hover"), hovered, 0.18);
        let show_controls = !t.controls_on_hover || !running || hovered;
        let controls = if fluid {
            ctx.animate_bool_with_time_and_easing(Id::unique("controls"), show_controls, 0.3, easing::cubic_out)
        } else {
            ctx.animate_bool_with_time(Id::unique("controls"), show_controls, 0.2)
        };

        // Fundo: clique inicia/pausa, arrastar move, botão direito abre as Preferências.
        let bg = ui.interact(full, Id::unique("background"), Sense::click_and_drag());
        if bg.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        } else if bg.clicked() && bg.interact_pointer_pos().is_some_and(|p| rect.contains(p)) {
            self.timer.toggle(now);
        }
        if ctx.input(|i| i.pointer.secondary_clicked()) {
            self.prefs.open = true;
        }

        let radius_f = self.corner_radius();
        let radius = CornerRadius::same(radius_f.round().min(255.0) as u8);
        let p = ui.painter().clone();
        if fluid {
            let (glow_a, glow_b) = self.glows(now, accent);
            let backdrop = Backdrop {
                base: t.background,
                glow_a,
                glow_b,
                opacity: t.opacity,
                dark,
                time: (now - self.started).as_secs_f32(),
                outline: t.outline,
            };
            backdrop.paint(&p, full, radius_f);
        } else {
            let [r, g, b, _] = t.background.to_array();
            p.rect_filled(full, radius, Color32::from_rgba_unmultiplied(r, g, b, (t.opacity * 255.0) as u8));
            if t.outline {
                p.rect_stroke(full.shrink(0.5), radius, Stroke::new(1.0, t.text.gamma_multiply(0.08)), StrokeKind::Inside);
            }
        }
        if let Some(until) = self.flash_until.filter(|u| now < *u) {
            let phase = (until - now).as_secs_f32() * std::f32::consts::TAU / 1.2;
            let a = 0.35 + 0.35 * phase.sin();
            p.rect_stroke(full.shrink(1.5 * s), radius, Stroke::new(3.0 * s, accent.gamma_multiply(a)), StrokeKind::Inside);
        }

        let progress = self.timer.progress(&self.settings, now);
        let roll = self.roll(now);
        let (time, prev) = (self.clock.shown.clone(), self.clock.prev.clone());
        let clock = |size: f32| FontId::new(size * s, fonts::clock());
        let track = if fluid { t.text.gamma_multiply(if dark { 0.09 } else { 0.12 }) } else { t.text.gamma_multiply(0.1) };
        let lights = if t.traffic_lights || fluid { hover } else { 0.0 };
        let cycles = self.settings.cycles_before_long.max(1);
        let label = self.timer.phase.label();
        // Modo Zen: detalhes (anel, nome da fase, bolinhas, barra) só aparecem com o mouse em cima.
        let zen = if t.zen { hover } else { 1.0 };
        let (track, accent_detail) = (track.gamma_multiply(zen), accent.gamma_multiply(zen));

        match t.layout {
            LayoutKind::Island => unreachable!("a Ilha é desenhada em draw_island"),
            LayoutKind::Ring => {
                let c = rect.center();
                if t.show_progress {
                    if fluid {
                        fluid::glow_ring(&p, c, 70.0 * s, 4.5 * s, progress, track, accent_detail);
                    } else {
                        w::ring(&p, c, 72.0 * s, 5.0 * s, progress, track, accent_detail);
                    }
                }
                let top = c + vec2(0.0, -31.0 * s);
                if t.show_phase {
                    if fluid {
                        fluid::chip(&p, top, Align2::CENTER_CENTER, label, 10.5 * s, accent, t.text, dark, (1.0 - lights) * zen);
                    } else {
                        w::caps_label(&p, top, Align2::CENTER_CENTER, label, 10.0 * s, t.secondary.gamma_multiply((1.0 - lights) * zen));
                    }
                }
                let d = if fluid { 16.0 * s } else { 11.0 * s };
                let group_w = if fluid { 3.0 * d + 0.9 * d } else { 3.0 * d + 1.2 * d };
                self.window_buttons(ui, pos2(c.x - group_w / 2.0, top.y), d, lights);
                let size = if fluid { 38.0 } else { 36.0 };
                w::clock_text(&p, c + vec2(0.0, -3.0 * s), Align2::CENTER_CENTER, &time, &prev, roll, clock(size), t.text);
                self.controls(ui, c + vec2(0.0, 27.0 * s), 31.0 * s, 24.0 * s, controls, false);
                if t.show_dots {
                    let dot_color = t.secondary.gamma_multiply(0.35 * zen);
                    w::dots(&p, c + vec2(0.0, 51.0 * s), cycles, self.timer.completed, 2.5 * s, accent_detail, dot_color);
                }
            }
            LayoutKind::Pill => {
                let left = rect.left() + 20.0 * s;
                let label_y = rect.top() + 18.0 * s;
                let label_alpha = (1.0 - lights) * zen;
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
                    bar(&p, rect, 20.0 * s, 8.0 * s, 3.0 * s, progress, track, accent_detail, fluid);
                }
            }
            LayoutKind::Minimal => {
                let c = rect.center() + vec2(0.0, -s);
                w::clock_text(&p, c, Align2::CENTER_CENTER, &time, &prev, roll, clock(30.0), t.text.gamma_multiply(1.0 - hover));
                if t.show_progress {
                    bar(&p, rect, (radius_f * 0.6).max(10.0 * s), 6.0 * s, 2.0 * s, progress, track, accent_detail, fluid);
                }
                self.controls(ui, c + vec2(-12.0 * s, 0.0), 28.0 * s, 24.0 * s, hover, true);
            }
        }
        if full.height() > rect.height() + 1.0 {
            let panel = Rect::from_min_max(pos2(full.left(), rect.bottom()), full.max);
            self.draw_music(ui, panel, now, hover);
        }

        // Bolinha discreta no canto quando há atualização; clique abre as Preferências → Sobre.
        if matches!(self.updater.state(), update::State::Available(_)) {
            let c = pos2(full.right() - radius_f.max(14.0 * s) * 0.55, full.top() + radius_f.max(14.0 * s) * 0.55);
            p.circle_filled(c, 6.0 * s, accent.gamma_multiply(0.25));
            p.circle_filled(c, 3.5 * s, accent);
            let r = ui.interact(Rect::from_center_size(c, Vec2::splat(16.0 * s)), Id::unique("update-dot"), Sense::click());
            if r.clicked() {
                self.prefs.open = true;
                self.prefs.tab = prefs::Tab::About;
            }
        }
    }

    /// Painel "tocando agora": música, artista e a linha atual da letra.
    /// Layout Ilha: pílula pequena que se abre num cartão ao passar o mouse (e quando uma fase termina).
    /// A janela cresce na hora; o desenho é que "abre" animado dentro dela, revelando o conteúdo.
    fn draw_island(&mut self, ui: &mut egui::Ui, now: Instant) {
        let ctx = ui.ctx().clone();
        let full = ui.max_rect();
        let t = self.theme.clone();
        let s = t.scale;
        let fluid = t.style == Style::Fluid;
        let dark = t.is_dark();
        let accent = self.shown_accent(now, t.accent(self.timer.phase));
        let running = self.timer.is_running();

        let closed = Rect::from_min_size(full.min, LayoutKind::Island.base_size() * s);
        let core_size = crate::theme::ISLAND_OPEN * s;
        let panel_h = self.music_panel_height();
        let open = Rect::from_min_size(full.min, core_size + vec2(0.0, panel_h));
        let k = ctx.animate_bool_with_time_and_easing(Id::unique("island"), self.island_open, 0.34, easing::cubic_out);
        self.island_t = k;
        let lerp = |a: f32, b: f32| a + (b - a) * k;
        let shape = Rect::from_min_max(
            closed.min,
            pos2(lerp(closed.right(), open.right()).min(full.right()), lerp(closed.bottom(), open.bottom()).min(full.bottom())),
        );

        // Abre com o mouse em cima (ou no fim de uma fase); fecha um instante depois que o mouse sai.
        let over = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| shape.expand(2.0).contains(p));
        let flashing = self.flash_until.is_some_and(|u| now < u);
        let was_open = self.island_open;
        if over || flashing {
            self.island_open = true;
            self.island_leave = None;
        } else if self.island_open {
            let left_at = *self.island_leave.get_or_insert(now);
            let wait = Duration::from_millis(450);
            if now - left_at >= wait {
                self.island_open = false;
                self.island_leave = None;
            } else {
                ctx.request_repaint_after(wait - (now - left_at));
            }
        }
        // A animação deste quadro foi calculada com o estado anterior: pede o próximo quadro para ela andar.
        if self.island_open != was_open {
            ctx.request_repaint();
        }

        let bg = ui.interact(shape, Id::unique("island-bg"), Sense::click_and_drag());
        if bg.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        } else if bg.clicked() {
            self.timer.toggle(now);
        }
        if ctx.input(|i| i.pointer.secondary_clicked()) {
            self.prefs.open = true;
        }

        // Fundo no formato atual da ilha.
        let radius_f = lerp(closed.height() / 2.0, (26.0 * s).min(t.corner_radius.max(14.0) * s));
        let radius = CornerRadius::same(radius_f.round().min(255.0) as u8);
        let p = ui.painter().clone();
        if fluid {
            let (glow_a, glow_b) = self.glows(now, accent);
            Backdrop {
                base: t.background,
                glow_a,
                glow_b,
                opacity: t.opacity,
                dark,
                time: (now - self.started).as_secs_f32(),
                outline: t.outline,
            }
            .paint(&p, shape, radius_f);
        } else {
            let [r, g, b, _] = t.background.to_array();
            p.rect_filled(shape, radius, Color32::from_rgba_unmultiplied(r, g, b, (t.opacity * 255.0) as u8));
            if t.outline {
                p.rect_stroke(shape.shrink(0.5), radius, Stroke::new(1.0, t.text.gamma_multiply(0.08)), StrokeKind::Inside);
            }
        }
        if let Some(until) = self.flash_until.filter(|u| now < *u) {
            let a = 0.35 + 0.35 * ((until - now).as_secs_f32() * std::f32::consts::TAU / 1.2).sin();
            p.rect_stroke(shape.shrink(1.5 * s), radius, Stroke::new(3.0 * s, accent.gamma_multiply(a)), StrokeKind::Inside);
        }

        let progress = self.timer.progress(&self.settings, now);
        let roll = self.roll(now);
        let (time, prev) = (self.clock.shown.clone(), self.clock.prev.clone());
        let clock = |size: f32| FontId::new(size * s, fonts::clock());
        let track = t.text.gamma_multiply(if dark { 0.12 } else { 0.15 });

        // Fechada: [capa ou bolinha da fase]  25:00  [anelzinho de progresso]
        let a0 = (1.0 - k * 1.6).max(0.0);
        if a0 > 0.01 {
            let cy = closed.center().y;
            let mut x = closed.left() + 14.0 * s;
            let art = if t.music_art && self.media.is_some() { self.art_texture(&ctx) } else { None };
            if let Some(tex) = art {
                let r = Rect::from_min_size(pos2(x - 4.0 * s, cy - 14.0 * s), Vec2::splat(28.0 * s));
                egui::Image::new(&tex).corner_radius(14.0 * s).tint(Color32::WHITE.gamma_multiply(a0)).paint_at(ui, r);
                x = r.right() + 9.0 * s;
            } else {
                p.circle_filled(pos2(x + 2.0 * s, cy), 4.5 * s, accent.gamma_multiply(a0));
                x += 15.0 * s;
            }
            w::clock_text(&p, pos2(x, cy), Align2::LEFT_CENTER, &time, &prev, roll, clock(21.0), t.text.gamma_multiply(a0));
            let rc = pos2(closed.right() - 22.0 * s, cy);
            let ring_color = if running { accent } else { accent.gamma_multiply(0.45) };
            w::ring(&p, rc, 8.5 * s, 2.6 * s, progress, track.gamma_multiply(a0), ring_color.gamma_multiply(a0));
        }

        // Aberta: o conteúdo é revelado pela forma crescendo (recorte) e aparece suavemente.
        let a1 = ((k - 0.25) / 0.75).clamp(0.0, 1.0);
        if a1 > 0.01 {
            let core = Rect::from_min_size(full.min, core_size);
            ui.set_clip_rect(shape);
            let p = ui.painter().clone();
            let label_y = core.top() + 22.0 * s;
            let left = core.left() + 20.0 * s;
            let mut x = left;
            if fluid {
                x = fluid::chip(&p, pos2(x - 4.0 * s, label_y), Align2::LEFT_CENTER, self.timer.phase.label(), 10.0 * s, accent, t.text, dark, a1).right();
            } else {
                x = w::caps_label(&p, pos2(x, label_y), Align2::LEFT_CENTER, self.timer.phase.label(), 10.0 * s, t.secondary.gamma_multiply(a1)).right();
            }
            let cycles = self.settings.cycles_before_long.max(1);
            let dots_w = (cycles as f32 * 9.0 - 4.0) * s;
            w::dots(&p, pos2(x + 8.0 * s + dots_w / 2.0, label_y), cycles, self.timer.completed, 2.5 * s, accent.gamma_multiply(a1), t.secondary.gamma_multiply(0.35 * a1));
            // Botões de janela no canto direito.
            let d = if fluid { 15.0 * s } else { 11.0 * s };
            let group_w = if fluid { 3.0 * d + 0.9 * d } else { 3.0 * d + 1.2 * d };
            self.window_buttons(ui, pos2(core.right() - 18.0 * s - group_w, label_y), d, a1);

            w::clock_text(&p, pos2(left, core.top() + 72.0 * s), Align2::LEFT_CENTER, &time, &prev, roll, clock(44.0), t.text.gamma_multiply(a1));
            self.controls(ui, pos2(core.right() - 66.0 * s, core.top() + 74.0 * s), 34.0 * s, 32.0 * s, a1, false);
            bar(&p, core, 20.0 * s, 16.0 * s, 4.0 * s, progress, track.gamma_multiply(a1), accent.gamma_multiply(a1), fluid);
            if panel_h > 0.0 {
                let panel = Rect::from_min_size(pos2(core.left(), core.bottom()), vec2(core.width(), panel_h));
                self.draw_music(ui, panel, now, a1);
            }
        }
    }

    /// Cores do gradiente do estilo Fluido: as da capa (se ligado e houver capa) ou as do tema,
    /// com uma transição suave quando a música muda.
    fn glows(&mut self, now: Instant, accent: Color32) -> (Color32, Color32) {
        let art = self.media.as_ref().and_then(|m| m.art()).filter(|_| self.theme.album_colors);
        let key = art.as_ref().map_or(0, |a| std::sync::Arc::as_ptr(a) as usize);
        let target = match &art {
            Some(a) => {
                let [c1, c2] = a.colors.map(|[r, g, b]| Color32::from_rgb(r, g, b));
                [crate::theme::mix(c1, accent, 0.15), c2]
            }
            None => {
                let (a, b) = self.theme.fluid_glows(accent);
                [a, b]
            }
        };
        let (from, to, at, old_key) = self.glow;
        let k = easing::cubic_in_out(((now - at).as_secs_f32() / ART_FADE).min(1.0));
        let shown = [w::lerp_color(from[0], to[0], k), w::lerp_color(from[1], to[1], k)];
        if old_key == usize::MAX {
            self.glow = (target, target, at, key); // primeira vez: sem transição
            return (target[0], target[1]);
        }
        if key != old_key || (art.is_none() && to != target) {
            self.glow = (shown, target, now, key);
        }
        (shown[0], shown[1])
    }

    fn glow_animating(&self, now: Instant) -> bool {
        self.theme.style == Style::Fluid && (now - self.glow.2).as_secs_f32() < ART_FADE
    }

    /// Textura da capa atual (carrega na GPU só quando a capa muda).
    fn art_texture(&mut self, ctx: &egui::Context) -> Option<egui::TextureHandle> {
        let art = self.media.as_ref()?.art()?;
        let key = std::sync::Arc::as_ptr(&art) as usize;
        if self.art_tex.as_ref().map(|(k, _)| *k) != Some(key) {
            let img = egui::ColorImage::from_rgba_unmultiplied(art.size, &art.rgba);
            let tex = ctx.load_texture("capa", img, egui::TextureOptions::LINEAR);
            self.art_tex = Some((key, tex));
        }
        self.art_tex.as_ref().map(|(_, t)| t.clone())
    }

    /// Painel "tocando agora": capa, música, artista e a letra.
    fn draw_music(&mut self, ui: &mut egui::Ui, panel: Rect, now: Instant, hover: f32) {
        let Some(media) = &self.media else { return };
        let (playback, lyrics) = media.snapshot();
        let tex = if self.theme.music_art { self.art_texture(ui.ctx()) } else { None };
        let Some(media) = &self.media else { return };
        let t = &self.theme;
        let s = t.scale;
        let p = ui.painter().clone();
        let inner = panel.shrink2(vec2(16.0 * s, 0.0));
        p.line_segment(
            [pos2(inner.left(), panel.top()), pos2(inner.right(), panel.top())],
            Stroke::new(1.0, t.text.gamma_multiply(0.08)),
        );

        let Some(pb) = playback else {
            p.text(panel.center(), Align2::CENTER_CENTER, "Nada tocando", FontId::proportional(11.0 * s), t.secondary);
            return;
        };

        // Capa à esquerda do nome e do artista (se houver e estiver ligada).
        let art_size = 40.0 * s;
        let mut text_left = inner.left();
        let top = inner.top() + if t.music_art { 10.0 } else { 8.0 } * s;
        if let Some(tex) = &tex {
            let r = Rect::from_min_size(pos2(inner.left(), top), Vec2::splat(art_size));
            p.rect_filled(r.translate(vec2(0.0, 1.5 * s)), CornerRadius::same((7.0 * s) as u8), Color32::from_black_alpha(50));
            egui::Image::new(tex).corner_radius(7.0 * s).paint_at(ui, r);
            text_left = r.right() + 10.0 * s;
        } else if t.music_art {
            // Sem capa: um quadradinho discreto no lugar, para o texto não pular quando a capa chegar.
            let r = Rect::from_min_size(pos2(inner.left(), top), Vec2::splat(art_size));
            p.rect_filled(r, CornerRadius::same((7.0 * s) as u8), t.text.gamma_multiply(0.06));
            let c = r.center();
            p.circle_stroke(c, art_size * 0.22, Stroke::new(1.5 * s, t.text.gamma_multiply(0.25)));
            p.circle_filled(c, art_size * 0.06, t.text.gamma_multiply(0.25));
            text_left = r.right() + 10.0 * s;
        }
        let text_w = inner.right() - text_left;

        // Nome da música em destaque, artista numa linha própria.
        let controls_w = if hover > 0.01 { 70.0 * s } else { 0.0 };
        let title_y = top + 2.0 * s;
        let title_job = one_line(&pb.track.title, FontId::proportional(12.5 * s), t.text, text_w - controls_w);
        p.galley(pos2(text_left, title_y), p.layout_job(title_job), t.text);

        let artist_y = title_y + 17.0 * s;
        let artist_color = w::lerp_color(t.secondary, t.text, 0.35);
        let tag = match &lyrics {
            // "≈": letra sem tempos, espalhada pela música (explicado nas Preferências).
            LyricsState::Found(l) if !l.synced => Some("≈"),
            LyricsState::Loading => Some("…"),
            _ => None,
        };
        let tag_w = if tag.is_some() { 16.0 * s } else { 0.0 };
        if !pb.track.artist.is_empty() {
            let artist_job = one_line(&pb.track.artist, FontId::proportional(11.0 * s), artist_color, text_w - tag_w);
            p.galley(pos2(text_left, artist_y), p.layout_job(artist_job), artist_color);
        }
        if let Some(tag) = tag {
            p.text(pos2(inner.right(), artist_y + 6.5 * s), Align2::RIGHT_CENTER, tag, FontId::proportional(11.0 * s), t.secondary.gamma_multiply(0.85));
        }

        if hover > 0.01 {
            let icon = 13.0 * s;
            let row = title_y + 8.0 * s;
            let mut x = inner.right() - icon / 2.0;
            let play = if pb.playing { Icon::Pause } else { Icon::Play };
            for (id, icon_kind, cmd) in [
                ("m-next", Icon::Skip, media::Command::Next),
                ("m-play", play, media::Command::PlayPause),
                ("m-prev", Icon::Previous, media::Command::Previous),
            ] {
                let r = Rect::from_center_size(pos2(x, row), Vec2::splat(icon));
                if w::icon_button(ui, r, id, icon_kind, t.text, hover).clicked() {
                    media.send(cmd);
                }
                x -= icon + 10.0 * s;
            }
        }

        // Letra.
        let body_top = (artist_y + 19.0 * s).max(if t.music_art { top + art_size + 8.0 * s } else { 0.0 });
        let body = Rect::from_min_max(pos2(inner.left(), body_top), pos2(inner.right(), panel.bottom() - 6.0 * s));
        let karaoke = t.lyrics_style == crate::theme::LyricsStyle::Karaoke;
        let animations = t.animations;
        let (text, color, idx, lines) = match &lyrics {
            LyricsState::Found(l) => {
                let pos = pb.position_now(now) + self.settings.lyrics_offset;
                let idx = l.line_at(pos);
                let line = idx.map(|i| l.lines[i].1.trim()).filter(|x| !x.is_empty());
                match line {
                    Some(line) => (line.to_owned(), t.text, idx, Some(l.clone())),
                    // Entre versos (ou antes da primeira linha). "…" porque nem toda fonte tem "♪".
                    None => ("…".to_owned(), t.secondary, idx, Some(l.clone())),
                }
            }
            LyricsState::NotFound => ("Sem letra para esta música".to_owned(), t.secondary.gamma_multiply(0.7), None, None),
            _ => (String::new(), t.text, None, None),
        };
        let (text_color, secondary) = (t.text, t.secondary);
        if self.lyric.0 != idx {
            self.lyric = (idx, now);
        }
        let k = if animations { easing::cubic_out(((now - self.lyric.1).as_secs_f32() / 0.35).min(1.0)) } else { 1.0 };

        match (karaoke, &lines) {
            (true, Some(l)) => {
                // Karaokê: a linha atual em destaque e a próxima embaixo. Na troca, a de cima sobe e some
                // e a próxima cresce até o lugar dela.
                let next_of = |i: Option<usize>| -> Option<&str> {
                    let start = i.map_or(0, |i| i + 1);
                    l.lines.get(start..)?.iter().map(|(_, x)| x.trim()).find(|x| !x.is_empty())
                };
                let cur_font = 13.5 * s;
                let next_font = 11.0 * s;
                let clip = p.with_clip_rect(body.expand2(vec2(4.0, 0.0)));
                // Linha que acabou de sair.
                if k < 1.0
                    && let Some(prev_i) = idx.and_then(|i| i.checked_sub(1))
                {
                    let prev = l.lines[prev_i].1.trim();
                    let job = wrapped(prev, FontId::proportional(cur_font), text_color.gamma_multiply(1.0 - k), body.width(), 2);
                    clip.galley(pos2(body.left(), body.top() - 18.0 * s * k), clip.layout_job(job), text_color);
                }
                // Linha atual (vindo da posição de "próxima").
                let size = next_font + (cur_font - next_font) * k;
                let c = w::lerp_color(secondary, color, k);
                let job = wrapped(&text, FontId::proportional(size), c, body.width(), 2);
                let g = clip.layout_job(job);
                let cur_h = g.size().y;
                let y = body.top() + 22.0 * s * (1.0 - k);
                clip.galley(pos2(body.left(), y), g, c);
                // Próxima linha, apagada.
                if let Some(next) = next_of(idx) {
                    let job = one_line(next, FontId::proportional(next_font), secondary.gamma_multiply(0.75 * k), body.width());
                    clip.galley(pos2(body.left(), y + cur_h + 3.0 * s + 8.0 * s * (1.0 - k)), clip.layout_job(job), secondary);
                }
            }
            _ => {
                let size = if color == text_color { 13.5 } else { 11.0 };
                let job = wrapped(&text, FontId::proportional(size * s), color.gamma_multiply(k), body.width(), 2);
                let galley = p.layout_job(job);
                let y = body.top() + 6.0 * s * (1.0 - k);
                p.galley(pos2(body.left(), y), galley, color);
            }
        }
    }

    /// Quanto falta para a próxima linha da letra (para redesenhar só nessa hora).
    fn next_lyric_change(&self, now: Instant) -> Option<Duration> {
        let (pb, lyrics) = self.media.as_ref()?.snapshot();
        let pb = pb.filter(|p| p.playing)?;
        let LyricsState::Found(l) = lyrics else { return None };
        let pos = pb.position_now(now) + self.settings.lyrics_offset;
        let since = (now - self.lyric.1).as_secs_f32();
        if self.theme.animations && since < 0.35 {
            return Some(Duration::from_millis(16));
        }
        let next = l.next_change(pos)?;
        Some(Duration::from_secs_f32((next - pos).clamp(0.0, 5.0)))
    }

    fn window_buttons(&mut self, ui: &mut egui::Ui, left_center: Pos2, d: f32, alpha: f32) {
        let action = if self.theme.style == Style::Fluid {
            fluid::window_buttons(ui, left_center, d, self.theme.text, self.theme.is_dark(), alpha)
        } else {
            w::traffic_lights(ui, left_center, d, alpha)
        };
        match action {
            TrafficAction::Close => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
            TrafficAction::Minimize => self.minimize(ui.ctx()),
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

/// Texto em uma linha só, com reticências se não couber.
fn one_line(text: &str, font: FontId, color: Color32, width: f32) -> egui::text::LayoutJob {
    wrapped(text, font, color, width, 1)
}

fn wrapped(text: &str, font: FontId, color: Color32, width: f32, rows: usize) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::simple(text.to_owned(), font, color, width.max(10.0));
    job.wrap.max_rows = rows;
    job.wrap.overflow_character = Some('…');
    job
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
    /// Roda mesmo com a janela escondida na bandeja: timer, bandeja, música e atualizações.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        if let Some(phase) = self.timer.tick(&self.settings, now) {
            self.on_phase_finished(ctx, now, phase);
        }
        self.sync_media(ctx);
        self.sync_tray(ctx, now);
        self.sync_updates(ctx, now);

        // Escondida, só precisa acordar para o timer (e para a bandeja acompanhar).
        if self.hidden && self.timer.is_running() {
            let left = self.timer.remaining(now);
            ctx.request_repaint_after(Duration::from_nanos(u64::from(left.subsec_nanos())).max(Duration::from_millis(50)));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = Instant::now();

        if self.first_frame {
            self.first_frame = false;
            render::mark_ok(self.mode);
            log::info!("Janela aberta com {}", self.mode.name());
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
        if flashing || animating || self.glow_animating(now) {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else if self.flash_until.take().is_some() {
            ctx.request_repaint();
        } else if let Some(wait) = self.next_lyric_change(now) {
            let timer_wait = self.timer.is_running().then(|| Duration::from_nanos(u64::from(self.timer.remaining(now).subsec_nanos())));
            ctx.request_repaint_after(timer_wait.map_or(wait, |t| t.min(wait)).max(Duration::from_millis(50)));
        } else if self.timer.is_running() {
            let left = self.timer.remaining(now);
            let to_next_second = Duration::from_nanos(u64::from(left.subsec_nanos()));
            ctx.request_repaint_after(to_next_second.max(Duration::from_millis(50)));
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, SETTINGS_KEY, &self.settings);
        eframe::set_value(storage, THEME_KEY, &self.theme);
        eframe::set_value(storage, STATS_KEY, &self.stats);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.relaunch {
            update::relaunch();
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        if self.mode.transparent() {
            [0.0; 4] // fundo transparente para os cantos arredondados
        } else {
            // Janela opaca (modo de compatibilidade): os cantos ficam da cor do fundo.
            self.theme.background.to_normalized_gamma_f32()
        }
    }
}
