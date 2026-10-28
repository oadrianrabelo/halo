//! Ícone na bandeja do sistema, com o progresso do timer desenhado nele.
//!
//! - Windows/macOS: `tray-icon`.
//! - Linux: StatusNotifierItem pelo D-Bus (`ksni`). Funciona no KDE, XFCE, Cinnamon, e no GNOME
//!   com a extensão AppIndicator. Sem isso, o Halo segue sem bandeja e minimiza normalmente.

use crate::icon;
use eframe::egui;
use std::sync::mpsc::{Receiver, Sender, channel};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Clique no ícone: mostra ou esconde a janela.
    Toggle,
    PlayPause,
    Skip,
    Preferences,
    Quit,
}

/// O que o ícone mostra. Só redesenha quando isto muda.
#[derive(Clone, PartialEq)]
pub struct View {
    /// Progresso em passos de 1/48, para não redesenhar o ícone a cada segundo.
    pub step: u8,
    pub accent: [u8; 3],
    pub running: bool,
    pub tooltip: String,
}

pub struct Tray {
    inner: platform::Inner,
    events: Receiver<Event>,
    last: Option<View>,
}

const SIZE: u32 = 32;

fn render(view: &View) -> Vec<u8> {
    let accent = if view.running { view.accent } else { view.accent.map(|c| (c as f32 * 0.6 + 60.0) as u8) };
    icon::render(SIZE, icon::Spec { background: None, accent, progress: view.step as f32 / 48.0, thickness: 0.16 })
}

impl Tray {
    /// Cria o ícone. `None` se o sistema não tiver bandeja disponível.
    pub fn new(ctx: &egui::Context) -> Option<Self> {
        let (tx, rx) = channel();
        let view = View { step: 0, accent: [0xFF, 0x45, 0x3A], running: false, tooltip: "Halo".into() };
        match platform::Inner::new(ctx.clone(), tx, &view) {
            Ok(inner) => {
                log::info!("Ícone da bandeja criado");
                Some(Self { inner, events: rx, last: Some(view) })
            }
            Err(e) => {
                log::warn!("Bandeja indisponível: {e}");
                None
            }
        }
    }

    pub fn set(&mut self, view: View) {
        if self.last.as_ref() == Some(&view) {
            return;
        }
        let icon_changed = self.last.as_ref().is_none_or(|l| l.step != view.step || l.accent != view.accent || l.running != view.running);
        self.inner.update(&view, icon_changed);
        self.last = Some(view);
    }

    pub fn events(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }
}

#[cfg(any(windows, target_os = "macos"))]
mod platform {
    use super::{Event, SIZE, View, render};
    use eframe::egui;
    use std::sync::mpsc::Sender;
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

    pub struct Inner {
        tray: TrayIcon,
    }

    impl Inner {
        pub fn new(ctx: egui::Context, tx: Sender<Event>, view: &View) -> Result<Self, String> {
            let menu = Menu::new();
            let items = [
                ("Mostrar / esconder", Event::Toggle),
                ("Iniciar / pausar", Event::PlayPause),
                ("Pular fase", Event::Skip),
                ("Preferências…", Event::Preferences),
            ];
            let mut ids = Vec::new();
            for (label, ev) in items {
                let item = MenuItem::new(label, true, None);
                ids.push((item.id().clone(), ev));
                menu.append(&item).map_err(|e| e.to_string())?;
            }
            menu.append(&PredefinedMenuItem::separator()).map_err(|e| e.to_string())?;
            let quit = MenuItem::new("Sair do Halo", true, None);
            ids.push((quit.id().clone(), Event::Quit));
            menu.append(&quit).map_err(|e| e.to_string())?;

            let menu_tx = tx.clone();
            let menu_ctx = ctx.clone();
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                if let Some((_, ev)) = ids.iter().find(|(id, _)| *id == e.id) {
                    let _ = menu_tx.send(*ev);
                    menu_ctx.request_repaint();
                }
            }));
            TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
                if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                    let _ = tx.send(Event::Toggle);
                    ctx.request_repaint();
                }
            }));

            let icon = Icon::from_rgba(render(view), SIZE, SIZE).map_err(|e| e.to_string())?;
            let tray = TrayIconBuilder::new()
                .with_icon(icon)
                .with_tooltip(&view.tooltip)
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()
                .map_err(|e| e.to_string())?;
            Ok(Self { tray })
        }

        pub fn update(&mut self, view: &View, icon_changed: bool) {
            if icon_changed && let Ok(icon) = Icon::from_rgba(render(view), SIZE, SIZE) {
                let _ = self.tray.set_icon(Some(icon));
            }
            let _ = self.tray.set_tooltip(Some(&view.tooltip));
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod platform {
    use super::{Event, SIZE, View, render};
    use eframe::egui;
    use ksni::blocking::{Handle, TrayMethods};
    use ksni::menu::{MenuItem, StandardItem};
    use std::sync::mpsc::Sender;

    struct Sni {
        view: View,
        pixmap: Vec<ksni::Icon>,
        tx: Sender<Event>,
        ctx: egui::Context,
    }

    impl Sni {
        fn send(&self, ev: Event) {
            let _ = self.tx.send(ev);
            self.ctx.request_repaint();
        }
    }

    /// O SNI quer ARGB em ordem de rede; o ícone é RGBA.
    fn pixmap(view: &View) -> Vec<ksni::Icon> {
        let rgba = render(view);
        let data = rgba.chunks_exact(4).flat_map(|p| [p[3], p[0], p[1], p[2]]).collect();
        vec![ksni::Icon { width: SIZE as i32, height: SIZE as i32, data }]
    }

    impl ksni::Tray for Sni {
        fn id(&self) -> String {
            "halo".into()
        }
        fn title(&self) -> String {
            "Halo".into()
        }
        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            self.pixmap.clone()
        }
        fn tool_tip(&self) -> ksni::ToolTip {
            ksni::ToolTip { title: self.view.tooltip.clone(), ..Default::default() }
        }
        fn activate(&mut self, _x: i32, _y: i32) {
            self.send(Event::Toggle);
        }
        fn menu(&self) -> Vec<MenuItem<Self>> {
            let item = |label: &str, ev: Event| {
                StandardItem { label: label.into(), activate: Box::new(move |t: &mut Self| t.send(ev)), ..Default::default() }.into()
            };
            vec![
                item("Mostrar / esconder", Event::Toggle),
                item("Iniciar / pausar", Event::PlayPause),
                item("Pular fase", Event::Skip),
                item("Preferências…", Event::Preferences),
                MenuItem::Separator,
                item("Sair do Halo", Event::Quit),
            ]
        }
    }

    pub struct Inner {
        handle: Handle<Sni>,
    }

    impl Inner {
        pub fn new(ctx: egui::Context, tx: Sender<Event>, view: &View) -> Result<Self, String> {
            let sni = Sni { view: view.clone(), pixmap: pixmap(view), tx, ctx };
            let handle = sni.spawn().map_err(|e| e.to_string())?;
            Ok(Self { handle })
        }

        pub fn update(&mut self, view: &View, icon_changed: bool) {
            let pixmap = icon_changed.then(|| pixmap(view));
            let view = view.clone();
            self.handle.update(move |t| {
                if let Some(p) = pixmap {
                    t.pixmap = p;
                }
                t.view = view;
            });
        }
    }
}

#[allow(dead_code)]
fn _assert_send(_: Sender<Event>) {}
