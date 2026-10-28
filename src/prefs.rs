//! Janela de Preferências, no estilo dos Ajustes do macOS.

use crate::app::App;
use crate::fluid::{self, Backdrop};
use crate::theme::{DigitWeight, FontChoice, LayoutKind, Style, Theme};
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
    Music,
    History,
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
    let (panel, card, control, text, weak, line) = match (theme.style, dark) {
        (Style::Classic, true) => (c(0x232325), c(0x2C2C2E), c(0x3A3A3C), c(0xF5F5F7), c(0x98989D), c(0x3D3D40)),
        (Style::Classic, false) => (c(0xECECEC), c(0xFFFFFF), c(0xE3E3E8), c(0x1D1D1F), c(0x86868B), c(0xDADADF)),
        // No Fluido as superfícies são vidro por cima do gradiente do fundo.
        (Style::Fluid, true) => (
            theme.background,
            Color32::from_white_alpha(10),
            Color32::from_white_alpha(22),
            theme.text,
            theme.secondary,
            Color32::from_white_alpha(16),
        ),
        (Style::Fluid, false) => (
            theme.background,
            Color32::from_white_alpha(150),
            Color32::from_black_alpha(14),
            theme.text,
            theme.secondary,
            Color32::from_black_alpha(14),
        ),
    };
    let accent = accent(theme);
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

/// Cor de destaque dos controles das Preferências.
pub fn accent(theme: &Theme) -> Color32 {
    match theme.style {
        Style::Classic => theme.long_break,
        // No claro as cores do gradiente são pastéis demais para controles.
        Style::Fluid if theme.is_dark() => theme.glow_a,
        Style::Fluid => theme.focus,
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let builder = egui::ViewportBuilder::default()
        .with_title("Preferências")
        .with_inner_size([480.0, 600.0])
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
        let t = &app.theme;
        if t.style == Style::Fluid {
            // Mesmo gradiente da janela principal, parado e mais calmo.
            let backdrop = Backdrop {
                base: t.background,
                glow_a: crate::theme::mix(t.background, t.glow_a, 0.55),
                glow_b: crate::theme::mix(t.background, t.glow_b, 0.55),
                opacity: 1.0,
                dark: t.is_dark(),
                time: 0.0,
            };
            backdrop.paint(ui.painter(), ui.max_rect(), 0.0);
        } else {
            ui.painter().rect_filled(ui.max_rect(), 0.0, ui.visuals().panel_fill);
        }
        egui::Frame::new().inner_margin(16.0).show(ui, |ui| {
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
        &[
            (Tab::Timer, "Timer"),
            (Tab::Appearance, "Aparência"),
            (Tab::Window, "Janela"),
            (Tab::Sound, "Som"),
            (Tab::Music, "Música"),
            (Tab::History, "Histórico"),
        ],
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
            Tab::Music => music_tab(app, ui),
            Tab::History => history_tab(app, ui),
        });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("Restaurar padrões").clicked() {
            match app.prefs.tab {
                Tab::Appearance | Tab::Window => {
                    app.theme = Theme::default();
                    app.settings.always_on_top = true;
                }
                Tab::History => {
                    app.settings.daily_goal = Settings::default().daily_goal;
                }
                Tab::Timer | Tab::Sound | Tab::Music => {
                    let keep_on_top = app.settings.always_on_top;
                    app.settings = Settings { always_on_top: keep_on_top, ..Settings::default() };
                    app.timer = Timer::new(&app.settings);
                }
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let quit = RichText::new("Sair do Halo").color(Color32::from_rgb(255, 69, 58));
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
    let accent = accent(&app.theme);
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
    let accent = accent(&app.theme);
    let t = &mut app.theme;

    group(ui, "ESTILO VISUAL", |rows| {
        rows.full(|ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                let w = (ui.available_width() - 12.0) / 2.0;
                for (style, name, desc) in [
                    (Style::Classic, "Clássico", "Sólido e discreto"),
                    (Style::Fluid, "Fluido", "Gradiente, vidro e brilho"),
                ] {
                    if style_card(ui, style, name, desc, t.style == style, accent, w).clicked() && t.style != style {
                        t.set_style(style);
                    }
                }
            });
        });
    });

    group(ui, "TEMA", |rows| {
        rows.full(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                for preset in t.style.presets() {
                    let selected = t.preset == preset.name;
                    if w::theme_swatch(ui, preset, t.style, selected, accent).clicked() {
                        t.apply_preset(preset);
                    }
                }
            });
        });
    });

    group(ui, "FORMA", |rows| {
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
        if t.style == Style::Fluid {
            color(rows, "Gradiente 1", &mut t.glow_a);
            color(rows, "Gradiente 2", &mut t.glow_b);
        }
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
        rows.row("Peso dos números", |ui| {
            let opts = [(DigitWeight::Thin, "Fino"), (DigitWeight::Regular, "Normal"), (DigitWeight::Bold, "Forte")];
            w::segmented(ui, &mut t.digit_weight, &opts, 180.0);
        });
    });
    if let Some(err) = &app.font_error {
        ui.colored_label(Color32::from_rgb(255, 69, 58), err);
    }

    group(ui, "DETALHES", |rows| {
        rows.row("Nome da fase", |ui| w::switch(ui, &mut t.show_phase, accent));
        rows.row("Bolinhas do ciclo", |ui| w::switch(ui, &mut t.show_dots, accent));
        rows.row("Progresso", |ui| w::switch(ui, &mut t.show_progress, accent));
        rows.row("Esconder controles enquanto roda", |ui| w::switch(ui, &mut t.controls_on_hover, accent));
        rows.row("Animações suaves", |ui| w::switch(ui, &mut t.animations, accent));
        if t.style == Style::Classic {
            rows.row("Botões estilo macOS", |ui| w::switch(ui, &mut t.traffic_lights, accent));
        }
    });
    hint(ui, "Animações suaves fazem os números rolarem e as cores mudarem devagar. Gastam um pouquinho mais de CPU por um instante a cada segundo.");
}

/// Cartão de escolha do estilo visual, com uma miniatura.
fn style_card(ui: &mut egui::Ui, style: Style, name: &str, desc: &str, selected: bool, accent: Color32, width: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 112.0), egui::Sense::click());
    let p = ui.painter();
    let preview = egui::Rect::from_min_size(rect.min, vec2(width, 70.0));
    let preset = &style.presets()[0];
    let [bg, fg, _, focus, ..] = preset.colors.map(crate::theme::hex);
    match style {
        Style::Classic => {
            p.rect_filled(preview, CornerRadius::same(12), bg);
            w::ring(p, preview.center(), 22.0, 4.0, 0.62, fg.gamma_multiply(0.12), focus);
        }
        Style::Fluid => {
            let [a, b] = preset.glow.map(crate::theme::hex);
            Backdrop { base: bg, glow_a: a, glow_b: b, opacity: 1.0, dark: true, time: 0.0 }.paint(p, preview, 12.0);
            fluid::glow_ring(p, preview.center(), 22.0, 3.5, 0.62, fg.gamma_multiply(0.1), focus);
        }
    }
    let stroke = if selected {
        Stroke::new(2.0, accent)
    } else if resp.hovered() {
        Stroke::new(1.0, accent.gamma_multiply(0.5))
    } else {
        Stroke::NONE
    };
    p.rect_stroke(preview.expand(2.5), CornerRadius::same(14), stroke, egui::StrokeKind::Outside);
    let v = ui.visuals();
    p.text(
        egui::pos2(rect.left() + 2.0, preview.bottom() + 14.0),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(13.5),
        v.strong_text_color(),
    );
    p.text(
        egui::pos2(rect.left() + 2.0, preview.bottom() + 32.0),
        egui::Align2::LEFT_CENTER,
        desc,
        egui::FontId::proportional(11.5),
        v.weak_text_color(),
    );
    resp
}

fn window_tab(app: &mut App, ui: &mut egui::Ui) {
    let accent = accent(&app.theme);
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
            ("Iniciar / pausar", vec!["Espaço"]),
            ("Reiniciar fase", vec!["R"]),
            ("Pular fase", vec!["S"]),
            ("Preferências", vec![cmd, ","]),
            ("Fechar Preferências", vec!["Esc"]),
            ("Sair", vec![cmd, "Q"]),
        ] {
            rows.row(action, |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for k in keys.iter().rev() {
                    w::keycap(ui, k);
                }
            });
        }
    });
    hint(ui, "Clique no timer para iniciar ou pausar, arraste para mover e use o botão direito para abrir as Preferências. Os botões que aparecem ao passar o mouse fecham, minimizam e abrem as Preferências.");
}

fn sound_tab(app: &mut App, ui: &mut egui::Ui) {
    let accent = accent(&app.theme);
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

fn music_tab(app: &mut App, ui: &mut egui::Ui) {
    let accent = accent(&app.theme);
    let s = &mut app.settings;
    group(ui, "TOCANDO AGORA", |rows| {
        rows.row("Mostrar a música tocando", |ui| w::switch(ui, &mut s.music, accent));
        rows.row("Só aparecer quando houver música", |ui| w::switch(ui, &mut s.music_only_when_playing, accent));
    });
    group(ui, "LETRAS", |rows| {
        rows.row("Letra sincronizada", |ui| w::switch(ui, &mut s.lyrics, accent));
        rows.row("Ajuste de tempo", |ui| {
            ui.add(egui::Slider::new(&mut s.lyrics_offset, -3.0..=3.0).step_by(0.1).show_value(false));
            ui.label(format!("{:+.1} s", s.lyrics_offset));
        });
    });

    // O que está sendo detectado agora, para a pessoa saber se está funcionando.
    let status = match app.media.as_ref().map(|m| m.snapshot()) {
        None => "Desligado.".to_owned(),
        Some((None, _)) => "Nada tocando agora.".to_owned(),
        Some((Some(pb), lyrics)) => {
            let letra = match lyrics {
                crate::media::LyricsState::Found(_) => "letra encontrada",
                crate::media::LyricsState::Loading => "buscando letra…",
                crate::media::LyricsState::NotFound => "sem letra sincronizada",
                crate::media::LyricsState::None => "letras desligadas",
            };
            format!("{} — {} ({letra})", pb.track.title, pb.track.artist)
        }
    };
    group(ui, "AGORA", |rows| {
        rows.full(|ui| ui.label(status));
    });

    let how = if cfg!(windows) {
        "Funciona com qualquer app que aparece nos controles de mídia do Windows: Spotify, navegador (YouTube Music, etc.), Música…"
    } else if cfg!(target_os = "macos") {
        "Funciona com Spotify e Música. Na primeira vez o macOS pede permissão para o Halo controlar o app."
    } else {
        "Funciona com qualquer player compatível com MPRIS: Spotify, navegadores, VLC, Rhythmbox, Elisa…"
    };
    hint(ui, how);
    hint(ui, "Letras de lrclib.net. Se a letra estiver adiantada ou atrasada, use o ajuste de tempo.");
}

fn history_tab(app: &mut App, ui: &mut egui::Ui) {
    use crate::stats::today;
    use chrono::{Datelike, Days};
    let accent = app.theme.accent(Phase::Focus);
    let today = today();
    let stats = &app.stats;
    let day = stats.day(today);
    let total = stats.total();
    let goal = app.settings.daily_goal.max(1);

    // Cartões de resumo.
    ui.add_space(4.0);
    ui.columns(3, |cols| {
        summary(&mut cols[0], "Hoje", &format!("{}", day.sessions), &format!("{} min de foco", day.minutes));
        let streak = stats.streak(today);
        summary(&mut cols[1], "Sequência", &format!("{streak}"), if streak == 1 { "dia" } else { "dias" });
        summary(&mut cols[2], "Total", &format!("{}", total.sessions), &format!("{:.1} h", total.minutes as f32 / 60.0));
    });

    group(ui, "META DIÁRIA", |rows| {
        rows.row("Focos por dia", |ui| w::stepper(ui, &mut app.settings.daily_goal, 1..=24, ""));
        rows.full(|ui| {
            let frac = day.sessions as f32 / goal as f32;
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 8.0), egui::Sense::hover());
            let p = ui.painter();
            p.rect_filled(rect, CornerRadius::same(4), ui.visuals().widgets.inactive.bg_fill);
            let mut done = rect;
            done.set_width(rect.width() * frac.min(1.0));
            if frac > 0.0 {
                p.rect_filled(done, CornerRadius::same(4), accent);
            }
            ui.add_space(4.0);
            let msg = if day.sessions >= goal {
                "Meta de hoje batida!".to_owned()
            } else {
                format!("{} de {goal} focos hoje", day.sessions)
            };
            ui.label(RichText::new(msg).size(11.5).color(ui.visuals().weak_text_color()));
        });
    });

    group(ui, "ÚLTIMOS 7 DIAS", |rows| {
        rows.full(|ui| {
            const NAMES: [&str; 7] = ["seg", "ter", "qua", "qui", "sex", "sáb", "dom"];
            let days: Vec<_> = (0..7).rev().map(|i| today - Days::new(i)).collect();
            let max = days.iter().map(|d| stats.day(*d).minutes).max().unwrap_or(0).max(1);
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 110.0), egui::Sense::hover());
            let p = ui.painter();
            let slot = rect.width() / 7.0;
            let weak = ui.visuals().weak_text_color();
            for (i, d) in days.iter().enumerate() {
                let m = stats.day(*d).minutes;
                let x = rect.left() + slot * (i as f32 + 0.5);
                let h = 70.0 * m as f32 / max as f32;
                let base = rect.bottom() - 20.0;
                let bar = egui::Rect::from_min_max(egui::pos2(x - slot * 0.22, base - h.max(3.0)), egui::pos2(x + slot * 0.22, base));
                let color = if *d == today { accent } else { accent.gamma_multiply(0.45) };
                p.rect_filled(bar, CornerRadius::same(4), if m == 0 { weak.gamma_multiply(0.25) } else { color });
                if m > 0 {
                    p.text(egui::pos2(x, bar.top() - 8.0), egui::Align2::CENTER_CENTER, format!("{m}"), egui::FontId::proportional(10.0), weak);
                }
                let name = NAMES[d.weekday().num_days_from_monday() as usize];
                p.text(egui::pos2(x, rect.bottom() - 8.0), egui::Align2::CENTER_CENTER, name, egui::FontId::proportional(11.0), weak);
            }
        });
    });

    group(ui, "ÚLTIMAS 16 SEMANAS", |rows| {
        rows.full(|ui| {
            // Grade estilo "contribuições": colunas = semanas, linhas = dias (seg a dom).
            let weeks = 16;
            let cell = ((ui.available_width() - 2.0 * (weeks as f32 - 1.0)) / weeks as f32).min(16.0);
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 7.0 * (cell + 2.0)), egui::Sense::hover());
            let p = ui.painter();
            let start = today - Days::new(u64::from(today.weekday().num_days_from_monday()) + 7 * (weeks - 1));
            let max = stats.days.values().map(|d| d.sessions).max().unwrap_or(1).max(1);
            let empty = ui.visuals().widgets.inactive.bg_fill;
            for wk in 0..weeks {
                for dow in 0..7 {
                    let d = start + Days::new(wk * 7 + dow);
                    if d > today {
                        continue;
                    }
                    let n = stats.day(d).sessions;
                    let c = if n == 0 { empty } else { accent.gamma_multiply(0.3 + 0.7 * n as f32 / max as f32) };
                    let min = rect.min + vec2(wk as f32 * (cell + 2.0), dow as f32 * (cell + 2.0));
                    p.rect_filled(egui::Rect::from_min_size(min, egui::Vec2::splat(cell)), CornerRadius::same(3), c);
                }
            }
        });
    });

    ui.add_space(6.0);
    let confirm = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::unique("confirm-clear")).unwrap_or(false));
    let label = if confirm { "Clique de novo para apagar tudo" } else { "Apagar histórico" };
    if ui.button(RichText::new(label).color(Color32::from_rgb(255, 69, 58))).clicked() {
        if confirm {
            app.stats = Default::default();
        }
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique("confirm-clear"), !confirm));
    }
}

/// Cartão com um número grande.
fn summary(ui: &mut egui::Ui, title: &str, value: &str, sub: &str) {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(11.0).color(ui.visuals().weak_text_color()));
            ui.label(RichText::new(value).size(26.0).strong());
            ui.label(RichText::new(sub).size(11.0).color(ui.visuals().weak_text_color()));
        });
}

fn hint(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(text).size(11.5).color(ui.visuals().weak_text_color()));
}
