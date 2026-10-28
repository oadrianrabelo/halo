//! Desenha o ícone do Halo (um anel de progresso) num buffer RGBA, sem dependências.
//! Usado pela janela, pela bandeja (com o progresso real) e pelo build.rs (ícone do .exe).

use std::f32::consts::TAU;

#[derive(Clone, Copy)]
pub struct Spec {
    /// Fundo em quadrado arredondado; `None` deixa transparente (bandeja).
    pub background: Option<[u8; 3]>,
    pub accent: [u8; 3],
    /// Fração do anel preenchida (0.0 a 1.0).
    pub progress: f32,
    /// Espessura do anel relativa ao tamanho.
    pub thickness: f32,
}

impl Spec {
    pub const APP: Spec = Spec { background: Some([0x1C, 0x1C, 0x1E]), accent: [0xFF, 0x45, 0x3A], progress: 0.72, thickness: 0.11 };
}

/// RGBA de `size`×`size`, com antisserrilhado por supersampling 4×4.
pub fn render(size: u32, spec: Spec) -> Vec<u8> {
    let s = size as f32;
    let c = s / 2.0;
    let (ring_r, width) = if spec.background.is_some() { (s * 0.31, s * spec.thickness) } else { (s * 0.38, s * spec.thickness) };
    let corner = s * 0.23;
    let progress = spec.progress.clamp(0.0, 1.0);
    let end_angle = progress * TAU;
    // Ponta do arco (em coordenadas da imagem, começando no topo, sentido horário).
    let end = (c + ring_r * end_angle.sin(), c - ring_r * end_angle.cos());
    let start = (c, c - ring_r);

    let in_bg = |x: f32, y: f32| {
        let dx = (x - c).abs() - (c - corner);
        let dy = (y - c).abs() - (c - corner);
        let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
        outside + dx.max(dy).min(0.0) <= corner
    };
    let ring_dist = |x: f32, y: f32| ((x - c).hypot(y - c) - ring_r).abs();
    let in_arc = |x: f32, y: f32| {
        if progress <= 0.0 {
            return false;
        }
        let near = |p: (f32, f32)| (x - p.0).hypot(y - p.1) <= width / 2.0;
        if near(start) || near(end) {
            return true;
        }
        let a = (x - c).atan2(-(y - c)).rem_euclid(TAU);
        ring_dist(x, y) <= width / 2.0 && a <= end_angle
    };

    let mut out = vec![0u8; (size * size * 4) as usize];
    const N: u32 = 4;
    for py in 0..size {
        for px in 0..size {
            // Acumula cor pré-multiplicada das amostras.
            let mut acc = [0.0f32; 4];
            for sy in 0..N {
                for sx in 0..N {
                    let x = px as f32 + (sx as f32 + 0.5) / N as f32;
                    let y = py as f32 + (sy as f32 + 0.5) / N as f32;
                    let mut col = [0.0f32; 4];
                    let mut over = |rgb: [u8; 3], a: f32| {
                        for i in 0..3 {
                            col[i] = col[i] * (1.0 - a) + rgb[i] as f32 / 255.0 * a;
                        }
                        col[3] = col[3] * (1.0 - a) + a;
                    };
                    if let Some(bg) = spec.background {
                        if !in_bg(x, y) {
                            continue;
                        }
                        over(bg, 1.0);
                        // Brilho suave em volta do anel.
                        let glow = (-(ring_dist(x, y) / (width * 1.2)).powi(2)).exp() * 0.35;
                        over(spec.accent, glow * if progress > 0.0 { 1.0 } else { 0.0 });
                    }
                    if ring_dist(x, y) <= width / 2.0 {
                        let track = if spec.background.is_some() { [255, 255, 255] } else { spec.accent };
                        over(track, if spec.background.is_some() { 0.12 } else { 0.3 });
                    }
                    if in_arc(x, y) {
                        over(spec.accent, 1.0);
                    }
                    if spec.background.is_some() && progress > 0.0 && (x - end.0).hypot(y - end.1) <= width * 0.32 {
                        over([255, 255, 255], 1.0);
                    }
                    for i in 0..4 {
                        acc[i] += if i < 3 { col[i] } else { col[3] };
                    }
                }
            }
            let n = (N * N) as f32;
            let a = acc[3] / n;
            let o = ((py * size + px) * 4) as usize;
            for i in 0..3 {
                // `col` já está "sobre" transparente, então divide pela cobertura para desfazer a pré-multiplicação.
                out[o + i] = if a > 0.0 { ((acc[i] / n / a) * 255.0).round().clamp(0.0, 255.0) as u8 } else { 0 };
            }
            out[o + 3] = (a * 255.0).round() as u8;
        }
    }
    out
}
