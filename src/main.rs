// Esconde o console no Windows em builds de release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod icons;
mod sound;
mod timer;

use eframe::egui::{self, Align, Color32, CornerRadius, Layout, RichText, Sense, Vec2};
use std::time::{Duration, Instant};
use icons::Icon;
use timer::{Phase, Settings, Timer, format_mmss};

const COMPACT: Vec2 = Vec2::new(200.0, 104.0);
const EXPANDED: Vec2 = Vec2::new(200.0, 268.0);
const SETTINGS_KEY: &str = "settings";

const BG: Color32 = Color32::from_rgb(24, 24, 28);
const FG: Color32 = Color32::from_rgb(230, 230, 235);
const DIM: Color32 = Color32::from_rgb(130, 130, 140);

fn phase_color(phase: Phase) -> Color32 {
    match phase {
        Phase::Focus => Color32::from_rgb(235, 95, 85),
        Phase::ShortBreak => Color32::from_rgb(80, 190, 130),
        Phase::LongBreak => Color32::from_rgb(90, 150, 230),
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Pomodoro")
            .with_inner_size(COMPACT)
            .with_resizable(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_taskbar(true),
        ..Default::default()
    };
    eframe::run_native("Pomodoro", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}

struct App {
    settings: Settings,
    timer: Timer,
    show_settings: bool,
    /// Pisca a janela por alguns segundos quando uma fase termina.
    flash_until: Option<Instant>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings: Settings = cc
            .storage
            .and_then(|s| eframe::get_value(s, SETTINGS_KEY))
            .unwrap_or_default();
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(FG);
        cc.egui_ctx.set_visuals(visuals);
        Self { timer: Timer::new(&settings), settings, show_settings: false, flash_until: None }
    }

    fn on_phase_finished(&mut self, ctx: &egui::Context, now: Instant) {
        if self.settings.sound {
            sound::notify();
        }
        self.flash_until = Some(now + Duration::from_secs(4));
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
            egui::UserAttentionType::Informational,
        ));
    }

    fn toggle_settings(&mut self, ctx: &egui::Context) {
        self.show_settings = !self.show_settings;
        let size = if self.show_settings { EXPANDED } else { COMPACT };
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        let color = phase_color(self.timer.phase);
        ui.horizontal(|ui| {
            ui.label(RichText::new(self.timer.phase.label()).color(color).strong());
            // Bolinhas indicando os focos já feitos no ciclo.
            icons::dots(ui, self.settings.cycles_before_long.max(1), self.timer.completed, color, DIM);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if small_button(ui, Icon::Close, "Fechar").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if small_button(ui, Icon::Settings, "Configurações").clicked() {
                    self.toggle_settings(ui.ctx());
                }
            });
        });
    }

    fn controls(&mut self, ui: &mut egui::Ui, now: Instant) {
        // Layout da direita para a esquerda: o primeiro botão fica mais à direita.
        if big_button(ui, Icon::Skip, "Pular fase (S)").clicked() {
            self.timer.skip(&self.settings, now);
        }
        if big_button(ui, Icon::Reset, "Reiniciar fase (R)").clicked() {
            self.timer.reset(&self.settings);
        }
        let play = if self.timer.is_running() { Icon::Pause } else { Icon::Play };
        if big_button(ui, play, "Iniciar/pausar (Espaço)").clicked() {
            self.timer.toggle(now);
        }
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let before = self.settings.clone();
        egui::Grid::new("settings").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            minutes_row(ui, "Foco", &mut self.settings.focus_min, 1..=180);
            minutes_row(ui, "Pausa", &mut self.settings.short_break_min, 1..=60);
            minutes_row(ui, "Pausa longa", &mut self.settings.long_break_min, 1..=120);
            ui.label("Ciclos");
            ui.add(egui::DragValue::new(&mut self.settings.cycles_before_long).range(1..=12));
            ui.end_row();
        });
        ui.checkbox(&mut self.settings.auto_start, "Iniciar próxima fase sozinho");
        ui.checkbox(&mut self.settings.sound, "Som ao terminar");

        // Se mudou a duração da fase atual e ela não começou, aplica na hora.
        if self.settings != before && !self.timer.is_running() {
            let full = before.duration(self.timer.phase);
            if self.timer.remaining(Instant::now()) == full {
                self.timer.reset(&self.settings);
            }
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context, now: Instant) {
        if ctx.memory(|m| m.focused().is_some()) {
            return; // não rouba teclas enquanto edita um campo
        }
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) {
                self.timer.toggle(now);
            }
            if i.key_pressed(egui::Key::R) {
                self.timer.reset(&self.settings);
            }
            if i.key_pressed(egui::Key::S) {
                self.timer.skip(&self.settings, now);
            }
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = Instant::now();

        if self.timer.tick(&self.settings, now).is_some() {
            self.on_phase_finished(&ctx, now);
        }
        self.shortcuts(&ctx, now);

        let flashing = self.flash_until.is_some_and(|t| now < t);
        let color = phase_color(self.timer.phase);
        let blink_on = flashing
            && self.flash_until.is_some_and(|t| ((t - now).as_millis() / 400).is_multiple_of(2));
        let bg = if blink_on { color.gamma_multiply(0.35) } else { BG };

        let panel = egui::Frame::new()
            .fill(bg)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(10.0);
        panel.show(ui, |ui| {
            ui.set_min_size(ui.available_size());

            // Arrastar em qualquer área vazia move a janela.
            let bg_resp = ui.interact(ui.max_rect(), ui.scope_id().with("drag"), Sense::drag());
            if bg_resp.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }

            self.header(ui);
            ui.horizontal(|ui| {
                let time = format_mmss(self.timer.remaining(now));
                ui.label(RichText::new(time).size(34.0).monospace().strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| self.controls(ui, now));
            });

            // Barra de progresso fina.
            let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 3.0), Sense::hover());
            ui.painter().rect_filled(rect, 1.5, DIM.gamma_multiply(0.3));
            let mut done = rect;
            done.set_width(rect.width() * self.timer.progress(&self.settings, now).clamp(0.0, 1.0));
            ui.painter().rect_filled(done, 1.5, color);

            if self.show_settings {
                ui.add_space(8.0);
                self.settings_ui(ui);
            }
        });

        // Quando rodando, acorda só a cada ~1s (ou mais rápido enquanto pisca).
        if flashing {
            ctx.request_repaint_after(Duration::from_millis(100));
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
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4] // fundo transparente para os cantos arredondados
    }
}

fn small_button(ui: &mut egui::Ui, icon: Icon, tip: &str) -> egui::Response {
    icons::button(ui, icon, 14.0, DIM, tip)
}

fn big_button(ui: &mut egui::Ui, icon: Icon, tip: &str) -> egui::Response {
    icons::button(ui, icon, 20.0, FG, tip)
}

fn minutes_row(ui: &mut egui::Ui, label: &str, value: &mut u32, range: std::ops::RangeInclusive<u32>) {
    ui.label(label);
    ui.add(egui::DragValue::new(value).range(range).suffix(" min"));
    ui.end_row();
}
