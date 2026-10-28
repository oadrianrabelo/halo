// Esconde o console no Windows em builds de release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod fluid;
mod fonts;
mod glass;
mod prefs;
mod sound;
mod theme;
mod timer;
mod widgets;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Pomodoro")
            .with_inner_size(theme::LayoutKind::Ring.base_size())
            .with_resizable(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_taskbar(true),
        ..Default::default()
    };
    eframe::run_native("Pomodoro", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
