// Esconde o console no Windows em builds de release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod fluid;
mod fonts;
mod glass;
mod http;
mod icon;
mod logging;
mod lyrics;
mod media;
mod prefs;
mod render;
mod sound;
mod stats;
mod theme;
mod timer;
mod tray;
mod update;
mod widgets;

use eframe::egui;

fn main() {
    logging::init();
    update::cleanup_old_binary();

    let icon = egui::IconData { rgba: icon::render(64, icon::Spec::APP), width: 64, height: 64 };
    let mut errors = Vec::new();

    // Tenta cada forma de desenhar a janela até uma funcionar. Computadores antigos às vezes
    // não têm OpenGL com transparência (ou OpenGL nenhum); aí caímos para DirectX ou software.
    for mode in render::attempts() {
        log::info!("Abrindo a janela com o renderizador {}", mode.name());
        render::mark_trying(mode);
        let mut options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("Halo")
                .with_app_id("halo")
                .with_icon(icon.clone())
                .with_inner_size(theme::LayoutKind::Ring.base_size())
                .with_resizable(false)
                .with_decorations(false)
                .with_transparent(mode.transparent())
                .with_always_on_top()
                .with_taskbar(true),
            ..Default::default()
        };
        mode.configure(&mut options);

        let result = eframe::run_native("Halo", options, Box::new(move |cc| Ok(Box::new(app::App::new(cc, mode)))));
        match result {
            Ok(()) => {
                log::info!("Halo fechado normalmente");
                return;
            }
            Err(e) => {
                log::error!("Renderizador {} falhou: {e}", mode.name());
                errors.push(format!("{}: {e}", mode.name()));
                render::mark_failed(mode);
            }
        }
    }

    logging::fatal(&format!("Não consegui abrir a janela do Halo neste computador.\n\n{}", errors.join("\n")));
    std::process::exit(1);
}
