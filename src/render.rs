//! Escolha do renderizador, com plano B, C e D para computadores antigos.
//!
//! Guarda num arquivo qual modo está sendo tentado. Se o Halo morrer no meio da tentativa
//! (driver travando, por exemplo), a próxima abertura já pula para o modo seguinte.
//! Para forçar um modo: variável de ambiente `HALO_RENDERER` (glow, glow-opaco, directx, software).

use crate::logging::data_dir;
use std::fs;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// OpenGL com janela transparente (cantos arredondados de verdade).
    Glow,
    /// OpenGL com janela opaca.
    GlowOpaque,
    /// DirectX 12 / Vulkan pela placa de vídeo.
    #[cfg(windows)]
    Gpu,
    /// Renderização por software (WARP do Windows). Funciona em qualquer Windows 10+.
    #[cfg(windows)]
    Software,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Glow => "glow",
            Mode::GlowOpaque => "glow-opaco",
            #[cfg(windows)]
            Mode::Gpu => "directx",
            #[cfg(windows)]
            Mode::Software => "software",
        }
    }

    pub fn transparent(self) -> bool {
        self == Mode::Glow
    }

    pub fn configure(self, options: &mut eframe::NativeOptions) {
        match self {
            Mode::Glow | Mode::GlowOpaque => options.renderer = eframe::Renderer::Glow,
            #[cfg(windows)]
            Mode::Gpu | Mode::Software => {
                use eframe::egui_wgpu::{WgpuSetup, wgpu};
                use std::sync::Arc;
                options.renderer = eframe::Renderer::Wgpu;
                let software = self == Mode::Software;
                if let WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup {
                    setup.native_adapter_selector = Some(Arc::new(move |adapters, _surface| {
                        let is_cpu = |a: &&wgpu::Adapter| a.get_info().device_type == wgpu::DeviceType::Cpu;
                        let pick = if software {
                            adapters.iter().find(is_cpu)
                        } else {
                            adapters.iter().find(|a| !is_cpu(a))
                        };
                        let names: Vec<String> = adapters.iter().map(|a| format!("{:?}", a.get_info())).collect();
                        log::info!("Placas encontradas: {names:?}");
                        pick.cloned().ok_or_else(|| "nenhuma placa adequada".to_owned())
                    }));
                }
            }
        }
    }
}

const ALL: &[Mode] = &[
    Mode::Glow,
    Mode::GlowOpaque,
    #[cfg(windows)]
    Mode::Gpu,
    #[cfg(windows)]
    Mode::Software,
];

fn marker() -> std::path::PathBuf {
    data_dir().join("renderer.txt")
}

/// Ordem de tentativas, começando pelo último modo que funcionou.
pub fn attempts() -> Vec<Mode> {
    if let Ok(forced) = std::env::var("HALO_RENDERER")
        && let Some(i) = ALL.iter().position(|m| m.name() == forced.trim())
    {
        return ALL[i..].to_vec();
    }
    let saved = fs::read_to_string(marker()).unwrap_or_default();
    let mut parts = saved.split_whitespace();
    let (state, name) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let start = match (state, ALL.iter().position(|m| m.name() == name)) {
        ("ok", Some(i)) => i,
        // Morreu no meio da última tentativa: pula esse modo.
        ("tentando" | "falhou", Some(i)) => {
            log::warn!("Última abertura com {name} não deu certo; tentando o próximo modo");
            (i + 1).min(ALL.len() - 1)
        }
        _ => 0,
    };
    ALL[start..].to_vec()
}

pub fn mark_trying(mode: Mode) {
    let _ = fs::write(marker(), format!("tentando {}", mode.name()));
}

pub fn mark_ok(mode: Mode) {
    let _ = fs::write(marker(), format!("ok {}", mode.name()));
}

pub fn mark_failed(mode: Mode) {
    let _ = fs::write(marker(), format!("falhou {}", mode.name()));
}

/// Volta a tentar do começo na próxima abertura (opção nas Preferências).
pub fn reset() {
    let _ = fs::remove_file(marker());
}
