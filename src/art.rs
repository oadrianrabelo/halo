//! Capa do álbum: carrega, reduz para um tamanho pequeno e tira as cores dominantes.

use std::time::Duration;

/// Tamanho guardado da capa (o painel mostra no máximo ~56 px).
const SIZE: u32 = 112;

pub struct Art {
    pub rgba: Vec<u8>,
    pub size: [usize; 2],
    /// Duas cores marcantes da capa (para o gradiente do estilo Fluido).
    pub colors: [[u8; 3]; 2],
}

/// De onde vem a capa.
#[allow(dead_code)] // cada sistema usa uma das formas
pub enum Source {
    Bytes(Vec<u8>),
    /// `file://…` ou `http(s)://…`.
    Url(String),
}

pub fn load(source: Source) -> Option<Art> {
    let bytes = match source {
        Source::Bytes(b) => b,
        Source::Url(url) => {
            if let Some(path) = url.strip_prefix("file://") {
                let path = percent_decode(path);
                std::fs::read(path).ok()?
            } else if url.starts_with("http://") || url.starts_with("https://") {
                crate::http::agent(Duration::from_secs(8))
                    .get(&url)
                    .call()
                    .ok()?
                    .body_mut()
                    .with_config()
                    .limit(10 * 1024 * 1024)
                    .read_to_vec()
                    .ok()?
            } else {
                return None;
            }
        }
    };
    let img = image::load_from_memory(&bytes).ok()?.thumbnail(SIZE, SIZE).to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let rgba = img.into_raw();
    let colors = palette(&rgba);
    Some(Art { rgba, size, colors })
}

/// Duas cores que representam a capa: a mais marcante e outra de matiz diferente.
/// Agrupa por matiz e pesa cada grupo por quanta saturação e brilho ele tem; cinzas quase não contam.
pub fn palette(rgba: &[u8]) -> [[u8; 3]; 2] {
    const BINS: usize = 18;
    let mut weight = [0.0f32; BINS];
    let mut sum = [[0.0f32; 3]; BINS];
    let mut avg = [0.0f32; 3];
    let mut n: f32 = 0.0;
    for p in rgba.chunks_exact(4) {
        if p[3] < 128 {
            continue;
        }
        let rgb = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0];
        let (h, s, v) = hsv(rgb);
        for i in 0..3 {
            avg[i] += rgb[i];
        }
        n += 1.0;
        let w = s * s * v * (1.0 - (v - 0.65).abs()); // prefere cores vivas e nem escuras nem estouradas
        if w < 0.02 {
            continue;
        }
        let bin = ((h / 360.0) * BINS as f32) as usize % BINS;
        weight[bin] += w;
        for i in 0..3 {
            sum[bin][i] += rgb[i] * w;
        }
    }
    let color = |bin: usize| -> [u8; 3] {
        let w = weight[bin].max(1e-6);
        sum[bin].map(|c| ((c / w).clamp(0.0, 1.0) * 255.0) as u8)
    };
    let mut order: Vec<usize> = (0..BINS).collect();
    order.sort_by(|a, b| weight[*b].total_cmp(&weight[*a]));
    let first = order[0];
    if weight[first] < 0.5 {
        // Capa quase sem cor (preto e branco): usa a média, clareada, e uma versão mais escura.
        let a = avg.map(|c| ((c / n.max(1.0)) * 255.0) as u8);
        let light = a.map(|c| c.saturating_add(70));
        return [light, a];
    }
    // Segunda cor: o grupo mais forte com matiz pelo menos ~60° longe da primeira.
    let far = |b: usize| {
        let d = (b as i32 - first as i32).rem_euclid(BINS as i32);
        d.min(BINS as i32 - d) >= 3
    };
    let second = order.iter().copied().find(|b| far(*b) && weight[*b] > weight[first] * 0.12);
    let c1 = color(first);
    let c2 = second.map(color).unwrap_or_else(|| c1.map(|c| (c as f32 * 0.55) as u8));
    [c1, c2]
}

fn hsv([r, g, b]: [f32; 3]) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max == 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

/// `file:///home/a%20b/capa.jpg` → `/home/a b/capa.jpg`.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_finds_two_distinct_colors() {
        // Metade vermelha, metade azul.
        let mut px = Vec::new();
        for i in 0..100 {
            px.extend_from_slice(if i < 60 { &[220, 40, 50, 255] } else { &[40, 80, 220, 255] });
        }
        let [a, b] = palette(&px);
        assert!(a[0] > 150 && a[2] < 100, "primeira deveria ser vermelha: {a:?}");
        assert!(b[2] > 150 && b[0] < 100, "segunda deveria ser azul: {b:?}");
    }

    #[test]
    fn palette_handles_grayscale() {
        let px: Vec<u8> = (0..100).flat_map(|i| [i as u8, i as u8, i as u8, 255]).collect();
        let [a, b] = palette(&px);
        assert!(a[0] >= b[0]);
    }

    #[test]
    fn decodes_file_urls() {
        assert_eq!(percent_decode("/home/a%20b/capa%C3%A9.jpg"), "/home/a b/capaé.jpg");
    }
}
