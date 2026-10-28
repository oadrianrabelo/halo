//! Janela de Preferências, no estilo dos Ajustes do macOS.

use crate::app::App;
use crate::theme::{FontChoice, LayoutKind, PRESETS, Theme};
use crate::timer::{Phase, Settings, Timer};
use crate::widgets::{self as w, group};
use crate::{glass, sound};
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, vec2};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Timer,
    Appearance,
    Window,
    Sound,
}

pub struct State {
    pub open: bool,
    pub tab: Tab,
    /// Caminho de fonte sendo digitado (só aplica ao confirmar).
    font_path: String,
}

impl Default for State {
    fn default() -> Self {
        Self { open: false, tab: Tab::Timer, font_path: String::new() }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum FontKind {
    System,
    Builtin,
    File,
}

/// Visual dos widgets do egui (usado só nas Preferências), claro ou escuro conforme o tema.
pub fn apply_visuals(ctx: &egui::Context, theme: &Theme) {
    let dark = theme.is_dark();
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    let c = |x: u32| crate::theme::hex(x);
    let (panel, card, control, text, weak, line) = if dark {
        (c(0x232325), c(0x2C2C2E), c(0x3A3A3C), c(0xF5F5F7), c(0x98989D), c(0x3D3D40))
    } else {
        (c(0xECECEC), c(0xFFFFFF), c(0xE3E3E8), c(0x1D1D1F), c(0x86868B), c(0xDADADF))
    };
    let accent = theme.long_break;
    v.panel_fill = panel;
    v.window_fill = card;
    v.extreme_bg_color = card;
    v.faint_bg_color = control;
    v.override_text_color = Some(text);
    v.weak_text_color = Some(weak);
    v.hyperlink_color = accent;
    v.selection.bg_fill = accent;
    v.selection.stroke = Stroke::new(1.0, w::contrast_on(accent));
    v.slider_trailing_fill = true;
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, line);
    for (wv, fill) in [
        (&mut v.widgets.inactive, control),
        (&mut v.widgets.hovered, w::lerp_color(control, text, 0.08)),
        (&mut v.widgets.active, w::lerp_color(control, text, 0.16)),
    ] {
        wv.bg_fill = fill;
        wv.weak_bg_fill = fill;
        wv.bg_stroke = Stroke::NONE;
        wv.corner_radius = CornerRadius::same(6);
        wv.expansion = 0.0;
    }
    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = vec2(8.0, 6.0);
        s.spacing.button_padding = vec2(10.0, 4.0);
        s.spacing.slider_width = 150.0;
        s.spacing.interact_size.y = 22.0;
    });
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let builder = egui::ViewportBuilder::default()
        .with_title("Preferências")
        .with_inner_size([420.0, 580.0])
        .with_min_inner_size([380.0, 420.0])
        .with_always_on_top();
    ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("prefs"), builder, |ui, _class| {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| {
            i.viewport().close_requested()
                || i.key_pressed(egui::Key::Escape)
                || (i.modifiers.command && i.key_pressed(egui::Key::W))
        }) {
            app.prefs.open = false;
        }
        egui::Frame::new().fill(ui.visuals().panel_fill).inner_margin(16.0).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            content(app, ui);
        });
    });
}

fn content(app: &mut App, ui: &mut egui::Ui) {
    let width = ui.available_width();
    w::segmented(
        ui,
        &mut app.prefs.tab,
        &[(Tab::Timer, "Timer"), (Tab::Appearance, "Aparência"), (Tab::Window, "Janela"), (Tab::Sound, "Som")],
        width,
    );
    ui.add_space(8.0);

    let footer_h = 40.0;
    egui::ScrollArea::vertical()
        .max_height(ui.available_height() - footer_h)
        .auto_shrink([false, false])
        .show(ui, |ui| match app.prefs.tab {
            Tab::Timer => timer_tab(app, ui),
            Tab::Appearance => appearance_tab(app, ui),
            Tab::Window => window_tab(app, ui),
            Tab::Sound => sound_tab(app, ui),
        });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("Restaurar padrões").clicked() {
            match app.prefs.tab {
                Tab::Appearance | Tab::Window => {
                    app.theme = Theme::default();
                    app.settings.always_on_top = true;
                }
                Tab::Timer | Tab::Sound => {
                    let keep_on_top = app.settings.always_on_top;
                    app.settings = Settings { always_on_top: keep_on_top, ..Settings::default() };
                    app.timer = Timer::new(&app.settings);
                }
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let quit = RichText::new("Sair do Pomodoro").color(Color32::from_rgb(255, 69, 58));
            if ui.button(quit).clicked() {
                ui.ctx().send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
            }
        });
    });
}

fn timer_tab(app: &mut App, ui: &mut egui::Ui) {
    let before = app.settings.clone();
    let s = &mut app.settings;
    group(ui, "DURAÇÃO", |rows| {
        rows.row("Foco", |ui| w::stepper(ui, &mut s.focus_min, 1..=180, " min"));
        rows.row("Pausa curta", |ui| w::stepper(ui, &mut s.short_break_min, 1..=60, " min"));
        rows.row("Pausa longa", |ui| w::stepper(ui, &mut s.long_break_min, 1..=120, " min"));
        rows.row("Focos até a pausa longa", |ui| w::stepper(ui, &mut s.cycles_before_long, 1..=12, ""));
    });
    let accent = app.theme.long_break;
    group(ui, "COMPORTAMENTO", |rows| {
        rows.row("Iniciar a próxima fase sozinho", |ui| w::switch(ui, &mut s.auto_start, accent));
    });
    hint(ui, "Dica: clique no timer para iniciar ou pausar e arraste para mover.");

    // Mudou a duração da fase atual e ela ainda não começou? Aplica na hora.
    if app.settings != before && !app.timer.is_running() {
        let full = before.duration(app.timer.phase);
        if app.timer.remaining(Instant::now()) == full {
            app.timer.reset(&app.settings);
        }
    }
}

fn appearance_tab(app: &mut App, ui: &mut egui::Ui) {
    let accent = app.theme.long_break;
    let t = &mut app.theme;

    group(ui, "TEMA", |rows| {
        rows.full(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                for preset in PRESETS {
                    let [bg, fg, _, focus, ..] = preset.colors.map(crate::theme::hex);
                    let selected = t.preset == preset.name;
                    if w::theme_swatch(ui, preset.name, bg, fg, focus, selected, accent).clicked() {
                        t.apply_preset(preset);
                    }
                }
            });
        });
    });

    group(ui, "ESTILO", |rows| {
        rows.row("Layout", |ui| {
            let opts = LayoutKind::ALL.map(|l| (l, l.label()));
            w::segmented(ui, &mut t.layout, &opts, 200.0);
        });
        rows.row("Tamanho", |ui| {
            ui.add(egui::Slider::new(&mut t.scale, 0.7..=2.0).show_value(false));
            ui.label(format!("{:.0}%", t.scale * 100.0));
        });
        rows.row("Cantos", |ui| ui.add(egui::Slider::new(&mut t.corner_radius, 0.0..=90.0).show_value(false)));
        rows.row("Opacidade", |ui| {
            ui.add(egui::Slider::new(&mut t.opacity, 0.2..=1.0).show_value(false));
            ui.label(format!("{:.0}%", t.opacity * 100.0));
        });
    });

    group(ui, "CORES", |rows| {
        let mut changed = false;
        let mut color = |rows: &mut w::Rows<'_>, label: &str, c: &mut Color32| {
            rows.row(label, |ui| changed |= ui.color_edit_button_srgba(c).changed());
        };
        color(rows, "Fundo", &mut t.background);
        color(rows, "Texto", &mut t.text);
        color(rows, "Texto secundário", &mut t.secondary);
        color(rows, Phase::Focus.label(), &mut t.focus);
        color(rows, Phase::ShortBreak.label(), &mut t.short_break);
        color(rows, Phase::LongBreak.label(), &mut t.long_break);
        if changed {
            t.preset = "Personalizado".into();
        }
    });

    group(ui, "FONTE", |rows| {
        let mut kind = match t.font {
            FontChoice::System => FontKind::System,
            FontChoice::Builtin => FontKind::Builtin,
            FontChoice::Custom(_) => FontKind::File,
        };
        rows.row("Fonte", |ui| {
            let opts = [(FontKind::System, "Sistema"), (FontKind::Builtin, "Embutida"), (FontKind::File, "Arquivo")];
            if w::segmented(ui, &mut kind, &opts, 210.0).changed() {
                t.font = match kind {
                    FontKind::System => FontChoice::System,
                    FontKind::Builtin => FontChoice::Builtin,
                    FontKind::File => FontChoice::Custom(app.prefs.font_path.clone()),
                };
            }
        });
        if kind == FontKind::File {
            rows.full(|ui| {
                ui.horizontal(|ui| {
                    let edit = egui::TextEdit::singleline(&mut app.prefs.font_path)
                        .hint_text("Caminho do .ttf ou .otf")
                        .desired_width(ui.available_width() - 70.0);
                    let resp = ui.add(edit);
                    let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("Aplicar").clicked() || enter {
                        t.font = FontChoice::Custom(app.prefs.font_path.trim().to_owned());
                    }
                });
            });
        }
        rows.row("Números finos", |ui| w::switch(ui, &mut t.thin_digits, accent));
    });
    if let Some(err) = &app.font_error {
        ui.colored_label(Color32::from_rgb(255, 69, 58), err);
    }

    group(ui, "MOSTRAR", |rows| {
        rows.row("Nome da fase", |ui| w::switch(ui, &mut t.show_phase, accent));
        rows.row("Bolinhas do ciclo", |ui| w::switch(ui, &mut t.show_dots, accent));
        rows.row("Progresso", |ui| w::switch(ui, &mut t.show_progress, accent));
        rows.row("Esconder controles enquanto roda", |ui| w::switch(ui, &mut t.controls_on_hover, accent));
        rows.row("Botões estilo macOS", |ui| w::switch(ui, &mut t.traffic_lights, accent));
    });
}

fn window_tab(app: &mut App, ui: &mut egui::Ui) {
    let accent = app.theme.long_break;
    group(ui, "JANELA", |rows| {
        rows.row("Sempre no topo", |ui| w::switch(ui, &mut app.settings.always_on_top, accent));
        rows.row("Efeito vidro", |ui| {
            ui.add_enabled_ui(glass::supported(), |ui| {
                let was = app.theme.glass;
                w::switch(ui, &mut app.theme.glass, accent);
                // Ligar o vidro com o fundo opaco não teria efeito visível.
                if app.theme.glass && !was && app.theme.opacity > 0.75 {
                    app.theme.opacity = 0.55;
                }
            });
        });
    });
    if !glass::supported() {
        hint(ui, "No Linux, o desfoque depende do compositor (KWin, Hyprland, picom…). Use a opacidade para deixar a janela translúcida.");
    } else if let Some(err) = &app.glass_error {
        ui.colored_label(Color32::from_rgb(255, 69, 58), err);
    }

    let cmd = if cfg!(target_os = "macos") { "⌘" } else { "Ctrl" };
    group(ui, "ATALHOS", |rows| {
        for (action, keys) in [
            ("Iniciar / pausar", "Espaço ou clique".to_owned()),
            ("Reiniciar fase", "R".to_owned()),
            ("Pular fase", "S".to_owned()),
            ("Preferências", format!("{cmd} + ,  ou botão direito")),
            ("Fechar Preferências", "Esc".to_owned()),
            ("Sair", format!("{cmd} + Q")),
        ] {
            rows.row(action, |ui| ui.label(RichText::new(keys).color(ui.visuals().weak_text_color())));
        }
    });
    hint(ui, "Com os botões estilo macOS: vermelho fecha, amarelo minimiza e verde abre as Preferências.");
}

fn sound_tab(app: &mut App, ui: &mut egui::Ui) {
    let accent = app.theme.long_break;
    let s = &mut app.settings;
    group(ui, "AVISO", |rows| {
        rows.row("Tocar som ao terminar", |ui| w::switch(ui, &mut s.sound, accent));
        rows.full(|ui| {
            ui.label("Arquivo de som");
            ui.horizontal(|ui| {
                let hint_text = if cfg!(windows) { "Padrão do sistema (.wav)" } else { "Padrão do sistema" };
                let edit = egui::TextEdit::singleline(&mut s.sound_file)
                    .hint_text(hint_text)
                    .desired_width(ui.available_width() - 70.0);
                ui.add(edit);
                if ui.button("Testar").clicked() {
                    sound::notify(&s.sound_file);
                }
            });
        });
    });
    hint(ui, "A janela também pisca e pede atenção na barra de tarefas quando uma fase termina.");
}

fn hint(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(text).size(11.5).color(ui.visuals().weak_text_color()));
}
