//! Letras sincronizadas do LRCLIB (https://lrclib.net), uma API aberta e sem chave.

use crate::media::Track;
use std::time::Duration;

const API: &str = "https://lrclib.net/api";
/// Permite apontar para outro servidor compatível (útil para testes).
fn api() -> String {
    std::env::var("HALO_LYRICS_API").unwrap_or_else(|_| API.to_owned())
}

#[derive(Debug, PartialEq)]
pub struct Lyrics {
    /// (segundos, texto), em ordem.
    pub lines: Vec<(f32, String)>,
}

impl Lyrics {
    /// Índice da linha que está sendo cantada em `pos` segundos.
    pub fn line_at(&self, pos: f32) -> Option<usize> {
        let i = self.lines.partition_point(|(t, _)| *t <= pos);
        i.checked_sub(1)
    }

    /// Quando começa a próxima linha (para saber quando redesenhar).
    pub fn next_change(&self, pos: f32) -> Option<f32> {
        self.lines.iter().map(|(t, _)| *t).find(|t| *t > pos)
    }
}

/// Lê o formato LRC: linhas como `[01:23.45] texto`, podendo ter várias marcas de tempo.
pub fn parse_lrc(text: &str) -> Lyrics {
    let mut lines = Vec::new();
    for raw in text.lines() {
        let mut rest = raw.trim();
        let mut times = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(end) = stripped.find(']') else { break };
            let tag = &stripped[..end];
            rest = stripped[end + 1..].trim_start();
            if let Some((m, s)) = tag.split_once(':')
                && let (Ok(m), Ok(s)) = (m.trim().parse::<f32>(), s.trim().parse::<f32>())
            {
                times.push(m * 60.0 + s);
            }
        }
        for t in times {
            lines.push((t, rest.to_owned()));
        }
    }
    lines.sort_by(|a, b| a.0.total_cmp(&b.0));
    Lyrics { lines }
}

/// Busca a letra sincronizada. `None` se não houver (ou se a música for instrumental).
pub fn fetch(track: &Track) -> Option<Lyrics> {
    if track.title.trim().is_empty() {
        return None;
    }
    let agent = crate::http::agent(Duration::from_secs(8));

    // 1) Busca exata (mais rápida e precisa).
    let mut req = agent
        .get(format!("{}/get", api()))
        .query("track_name", &track.title)
        .query("artist_name", &track.artist);
    if !track.album.is_empty() {
        req = req.query("album_name", &track.album);
    }
    if let Some(d) = track.duration {
        req = req.query("duration", (d.round() as u32).to_string());
    }
    if let Some(l) = req.call().ok().and_then(|mut r| r.body_mut().read_to_string().ok()).and_then(|b| from_json(&b)) {
        return Some(l);
    }

    // 2) Busca aproximada: pega o primeiro resultado com letra sincronizada.
    let body = agent
        .get(format!("{}/search", api()))
        .query("track_name", &track.title)
        .query("artist_name", &track.artist)
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()?;
    let results: Vec<serde_json::Value> = serde_json::from_str(&body).ok()?;
    results.iter().find_map(from_value)
}

fn from_json(body: &str) -> Option<Lyrics> {
    from_value(&serde_json::from_str(body).ok()?)
}

fn from_value(v: &serde_json::Value) -> Option<Lyrics> {
    let synced = v.get("syncedLyrics")?.as_str()?;
    let lyrics = parse_lrc(synced);
    (!lyrics.lines.is_empty()).then_some(lyrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lrc() {
        let l = parse_lrc("[00:01.50] Olá\n[ar: alguém]\n[00:03.00][00:10.00] Refrão\n[00:05.00]\n");
        assert_eq!(
            l.lines,
            vec![(1.5, "Olá".into()), (3.0, "Refrão".into()), (5.0, String::new()), (10.0, "Refrão".into())]
        );
        assert_eq!(l.line_at(0.5), None);
        assert_eq!(l.line_at(3.2), Some(1));
        assert_eq!(l.next_change(3.2), Some(5.0));
    }
}
