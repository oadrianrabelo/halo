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
    /// Pílula pequena que se expande num cartão ao passar o mouse.
    Island,
}

/// Tamanho da Ilha aberta (sem o painel de música), em escala 1.
pub const ISLAND_OPEN: Vec2 = Vec2::new(300.0, 152.0);

impl LayoutKind {
    pub const ALL: [LayoutKind; 4] = [LayoutKind::Ring, LayoutKind::Pill, LayoutKind::Minimal, LayoutKind::Island];

    pub fn label(self) -> &'static str {
        match self {
            LayoutKind::Ring => "Anel",
            LayoutKind::Pill => "Pílula",
            LayoutKind::Minimal => "Mínimo",
            LayoutKind::Island => "Ilha",
        }
    }

    /// Tamanho da janela em escala 1.
    pub fn base_size(self) -> Vec2 {
        match self {
            LayoutKind::Ring => Vec2::new(176.0, 176.0),
            LayoutKind::Pill => Vec2::new(252.0, 68.0),
            LayoutKind::Minimal => Vec2::new(136.0, 52.0),
            // Ilha fechada; aberta ela usa `ISLAND_OPEN`.
            LayoutKind::Island => Vec2::new(172.0, 44.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Style {
    /// Superfícies sólidas e discretas, com botões no estilo macOS.
    Classic,
    /// Gradientes vivos, vidro, brilho e animações (inspirado em Arc, Raycast, Dia).
    Fluid,
}

impl Style {
    pub fn presets(self) -> &'static [Preset] {
        match self {
            Style::Classic => PRESETS,
            Style::Fluid => FLUID_PRESETS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DigitWeight {
    Thin,
    Regular,
    Bold,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FontChoice {
    /// Fonte de interface do sistema (SF no Mac, Segoe no Windows, Inter/Cantarell/... no Linux).
    System,
    /// Fonte embutida no egui.
    Builtin,
    /// Inter, embutida no Halo (a mesma em todos os sistemas).
    Inter,
    /// Caminho para um .ttf/.otf qualquer.
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub style: Style,
    pub preset: String,
    pub background: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub focus: Color32,
    pub short_break: Color32,
    pub long_break: Color32,
    /// Cores do gradiente no estilo Fluido.
    pub glow_a: Color32,
    pub glow_b: Color32,
    /// Opacidade do fundo (0.2 a 1.0).
    pub opacity: f32,
    /// Raio dos cantos. No máximo vira círculo/pílula.
    pub corner_radius: f32,
    /// Escala da janela (0.7 a 2.0).
    pub scale: f32,
    pub layout: LayoutKind,
    pub font: FontChoice,
    /// Peso dos números (usa a variação da fonte do sistema, quando existir).
    pub digit_weight: DigitWeight,
    /// Números rolando e cores em transição. Gasta um pouco mais de CPU.
    pub animations: bool,
    pub show_phase: bool,
    pub show_dots: bool,
    pub show_progress: bool,
    /// Esconde os controles enquanto o timer roda e o mouse está fora.
    pub controls_on_hover: bool,
    /// Botões vermelho/amarelo/verde no estilo macOS.
    pub traffic_lights: bool,
    /// Desfoque nativo atrás da janela (Windows/macOS).
    pub glass: bool,
    /// Contorno fino em volta da janela.
    pub outline: bool,
    /// Modo Zen: só o tempo fica na tela; o resto aparece ao passar o mouse.
    pub zen: bool,
    /// Capa do álbum no painel de música.
    pub music_art: bool,
    /// No estilo Fluido, o gradiente usa as cores da capa da música tocando.
    pub album_colors: bool,
    pub lyrics_style: LyricsStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LyricsStyle {
    /// Só a linha atual.
    Line,
    /// Linha atual em destaque e a próxima embaixo, rolando.
    Karaoke,
}

impl Default for Theme {
    fn default() -> Self {
        let mut t = Self {
            style: Style::Classic,
            preset: String::new(),
            background: Color32::BLACK,
            text: Color32::WHITE,
            secondary: Color32::GRAY,
            focus: Color32::RED,
            short_break: Color32::GREEN,
            long_break: Color32::BLUE,
            glow_a: Color32::RED,
            glow_b: Color32::BLUE,
            opacity: 0.92,
            corner_radius: 22.0,
            scale: 1.0,
            layout: LayoutKind::Ring,
            font: FontChoice::System,
            digit_weight: DigitWeight::Thin,
            animations: false,
            show_phase: true,
            show_dots: true,
            show_progress: true,
            controls_on_hover: true,
            traffic_lights: true,
            glass: false,
            outline: true,
            zen: false,
            music_art: true,
            album_colors: true,
            lyrics_style: LyricsStyle::Karaoke,
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
        [self.glow_a, self.glow_b] = p.glow.map(hex);
    }

    /// Troca de estilo, ajustando os padrões que combinam com ele.
    pub fn set_style(&mut self, style: Style) {
        self.style = style;
        self.apply_preset(&style.presets()[0]);
        match style {
            Style::Classic => {
                self.corner_radius = 22.0;
                self.opacity = 0.92;
                self.digit_weight = DigitWeight::Thin;
                self.animations = false;
                self.traffic_lights = true;
            }
            Style::Fluid => {
                self.font = FontChoice::Inter;
                self.corner_radius = 30.0;
                self.opacity = 0.97;
                self.digit_weight = DigitWeight::Bold;
                self.animations = true;
            }
        }
    }

    /// Cor que "dirige" o visual Fluido: o gradiente puxa para a cor da fase.
    pub fn fluid_glows(&self, accent: Color32) -> (Color32, Color32) {
        (mix(self.glow_a, accent, 0.45), mix(self.glow_b, accent, 0.2))
    }
}

pub struct Preset {
    pub name: &'static str,
    /// fundo, texto, secundário, foco, pausa, pausa longa
    pub colors: [u32; 6],
    /// As duas cores do gradiente (estilo Fluido).
    pub glow: [u32; 2],
}

pub const PRESETS: &[Preset] = &[
    Preset { name: "Grafite", colors: [0x1C1C1E, 0xF5F5F7, 0x8E8E93, 0xFF453A, 0x30D158, 0x0A84FF], glow: [0xFF453A, 0x0A84FF] },
    Preset { name: "Claro", colors: [0xF5F5F7, 0x1D1D1F, 0x86868B, 0xFF3B30, 0x34C759, 0x007AFF], glow: [0xFF3B30, 0x007AFF] },
    Preset { name: "Meia-noite", colors: [0x0B1020, 0xE6E9F2, 0x6B7394, 0xFF6B8B, 0x5EEAD4, 0x818CF8], glow: [0xFF6B8B, 0x818CF8] },
    Preset { name: "Areia", colors: [0xF4ECD8, 0x4A3F35, 0x9C8F80, 0xD9622B, 0x6A994E, 0x3D7EA6], glow: [0xD9622B, 0x3D7EA6] },
    Preset { name: "Nord", colors: [0x2E3440, 0xECEFF4, 0x7B88A1, 0xBF616A, 0xA3BE8C, 0x88C0D0], glow: [0xBF616A, 0x88C0D0] },
    Preset { name: "Rosé", colors: [0x191724, 0xE0DEF4, 0x6E6A86, 0xEB6F92, 0x9CCFD8, 0xC4A7E7], glow: [0xEB6F92, 0xC4A7E7] },
    Preset { name: "Matcha", colors: [0xEEF2E6, 0x2F3E2E, 0x8A9A84, 0xC8553D, 0x588157, 0x3A5A40], glow: [0xC8553D, 0x3A5A40] },
    Preset { name: "Lavanda", colors: [0xF3F0FA, 0x2E2A3D, 0x8D87A3, 0xE0607E, 0x4FA586, 0x7B61FF], glow: [0xE0607E, 0x7B61FF] },
];

pub const FLUID_PRESETS: &[Preset] = &[
    Preset { name: "Aurora", colors: [0x15121F, 0xFFFFFF, 0xA9A3BF, 0xFF5C8A, 0x3DDC97, 0x7C6CFF], glow: [0xFF5C8A, 0x6B4DFF] },
    Preset { name: "Carvão", colors: [0x0F0F11, 0xF4F4F5, 0x8B8B93, 0xFF6363, 0x59D499, 0x56C2FF], glow: [0xFF6363, 0x2A2A35] },
    Preset { name: "Algodão", colors: [0xF7F4FF, 0x221F2E, 0x8E88A3, 0xF45D8C, 0x2FB37E, 0x5B7CFA], glow: [0xFFC2DE, 0xB9CCFF] },
    Preset { name: "Oceano", colors: [0x06131D, 0xEAF7FF, 0x7FA3B8, 0xFF7A6B, 0x2DE2A6, 0x00B4FF], glow: [0x00B4FF, 0x2DE2A6] },
    Preset { name: "Brasa", colors: [0x1A0E10, 0xFFF4EE, 0xB89A90, 0xFF7A45, 0x8FD18B, 0xFFB347], glow: [0xFF7A45, 0xD6336C] },
    Preset { name: "Névoa", colors: [0xF2F4F7, 0x1E2430, 0x8790A0, 0xEF5B5B, 0x34B37A, 0x4C7DF0], glow: [0xD4DEFF, 0xE9E3F8] },
    Preset { name: "Floresta", colors: [0x0C1712, 0xEFFAF3, 0x86A394, 0xF2875A, 0x5BD69B, 0x3FB7C9], glow: [0x1FAF75, 0x0E6C5A] },
    Preset { name: "Lilás", colors: [0x1B1528, 0xF7F2FF, 0xA79BC4, 0xF472B6, 0x6EE7B7, 0xA78BFA], glow: [0xA78BFA, 0xF472B6] },
];

/// Mistura simples entre duas cores (sem alfa).
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

pub const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}
