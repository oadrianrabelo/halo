//! Aparência: cores, layout, fonte e tudo que dá para personalizar.

use crate::timer::Phase;
use eframe::egui::{Color32, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutKind {
    /// Anel de progresso em volta do relógio.
    Ring,
    /// Pílula horizontal com barra de progresso.
    Pill,
    /// Só os números.
    Minimal,
}

impl LayoutKind {
    pub const ALL: [LayoutKind; 3] = [LayoutKind::Ring, LayoutKind::Pill, LayoutKind::Minimal];

    pub fn label(self) -> &'static str {
        match self {
            LayoutKind::Ring => "Anel",
            LayoutKind::Pill => "Pílula",
            LayoutKind::Minimal => "Mínimo",
        }
    }

    /// Tamanho da janela em escala 1.
    pub fn base_size(self) -> Vec2 {
        match self {
            LayoutKind::Ring => Vec2::new(176.0, 176.0),
            LayoutKind::Pill => Vec2::new(252.0, 68.0),
            LayoutKind::Minimal => Vec2::new(136.0, 52.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FontChoice {
    /// Fonte de interface do sistema (SF no Mac, Segoe no Windows, Inter/Cantarell/... no Linux).
    System,
    /// Fonte embutida no egui.
    Builtin,
    /// Caminho para um .ttf/.otf qualquer.
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub preset: String,
    pub background: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub focus: Color32,
    pub short_break: Color32,
    pub long_break: Color32,
    /// Opacidade do fundo (0.2 a 1.0).
    pub opacity: f32,
    /// Raio dos cantos. No máximo vira círculo/pílula.
    pub corner_radius: f32,
    /// Escala da janela (0.7 a 2.0).
    pub scale: f32,
    pub layout: LayoutKind,
    pub font: FontChoice,
    /// Usa a variação fina da fonte nos números, quando existir.
    pub thin_digits: bool,
    pub show_phase: bool,
    pub show_dots: bool,
    pub show_progress: bool,
    /// Esconde os controles enquanto o timer roda e o mouse está fora.
    pub controls_on_hover: bool,
    /// Botões vermelho/amarelo/verde no estilo macOS.
    pub traffic_lights: bool,
    /// Desfoque nativo atrás da janela (Windows/macOS).
    pub glass: bool,
}

impl Default for Theme {
    fn default() -> Self {
        let mut t = Self {
            preset: String::new(),
            background: Color32::BLACK,
            text: Color32::WHITE,
            secondary: Color32::GRAY,
            focus: Color32::RED,
            short_break: Color32::GREEN,
            long_break: Color32::BLUE,
            opacity: 0.92,
            corner_radius: 22.0,
            scale: 1.0,
            layout: LayoutKind::Ring,
            font: FontChoice::System,
            thin_digits: true,
            show_phase: true,
            show_dots: true,
            show_progress: true,
            controls_on_hover: true,
            traffic_lights: true,
            glass: false,
        };
        t.apply_preset(&PRESETS[0]);
        t
    }
}

impl Theme {
    pub fn accent(&self, phase: Phase) -> Color32 {
        match phase {
            Phase::Focus => self.focus,
            Phase::ShortBreak => self.short_break,
            Phase::LongBreak => self.long_break,
        }
    }

    pub fn is_dark(&self) -> bool {
        let [r, g, b, _] = self.background.to_array();
        (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) < 128.0
    }

    pub fn window_size(&self) -> Vec2 {
        self.layout.base_size() * self.scale
    }

    pub fn apply_preset(&mut self, p: &Preset) {
        self.preset = p.name.to_owned();
        [self.background, self.text, self.secondary, self.focus, self.short_break, self.long_break] =
            p.colors.map(hex);
    }
}

pub struct Preset {
    pub name: &'static str,
    /// fundo, texto, secundário, foco, pausa, pausa longa
    pub colors: [u32; 6],
}

pub const PRESETS: &[Preset] = &[
    Preset { name: "Grafite", colors: [0x1C1C1E, 0xF5F5F7, 0x8E8E93, 0xFF453A, 0x30D158, 0x0A84FF] },
    Preset { name: "Claro", colors: [0xF5F5F7, 0x1D1D1F, 0x86868B, 0xFF3B30, 0x34C759, 0x007AFF] },
    Preset { name: "Meia-noite", colors: [0x0B1020, 0xE6E9F2, 0x6B7394, 0xFF6B8B, 0x5EEAD4, 0x818CF8] },
    Preset { name: "Areia", colors: [0xF4ECD8, 0x4A3F35, 0x9C8F80, 0xD9622B, 0x6A994E, 0x3D7EA6] },
    Preset { name: "Nord", colors: [0x2E3440, 0xECEFF4, 0x7B88A1, 0xBF616A, 0xA3BE8C, 0x88C0D0] },
    Preset { name: "Rosé", colors: [0x191724, 0xE0DEF4, 0x6E6A86, 0xEB6F92, 0x9CCFD8, 0xC4A7E7] },
    Preset { name: "Matcha", colors: [0xEEF2E6, 0x2F3E2E, 0x8A9A84, 0xC8553D, 0x588157, 0x3A5A40] },
    Preset { name: "Lavanda", colors: [0xF3F0FA, 0x2E2A3D, 0x8D87A3, 0xE0607E, 0x4FA586, 0x7B61FF] },
];

pub const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}
