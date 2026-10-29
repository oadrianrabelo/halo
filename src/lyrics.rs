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
    /// `false` quando a letra veio sem tempos e foi espalhada pela duração da música.
    pub synced: bool,
    /// De onde veio (mostrado nas Preferências).
    pub source: &'static str,
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

/// O que buscar e onde.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Aceita letra sem tempos (fica com o tempo aproximado).
    pub plain: bool,
    /// Chave da API do Vagalume. Vazia = não usa o Vagalume.
    pub vagalume_key: String,
}

/// Lê o formato LRC: linhas como `[01:23.45] texto`, podendo ter várias marcas de tempo.
pub fn parse_lrc(text: &str) -> Vec<(f32, String)> {
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
    lines
}

/// Espalha uma letra sem tempos pela música: cada linha ganha um tempo proporcional ao tamanho
/// dela, entre ~8% e ~92% da duração (pula introdução e final). Fica aproximado, mas acompanha.
pub fn approximate(text: &str, duration: Option<f32>) -> Vec<(f32, String)> {
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let duration = duration.filter(|d| *d > 30.0).unwrap_or(210.0);
    let (start, end) = (duration * 0.08, duration * 0.92);
    // Peso mínimo para linhas curtas ("Oh!") não passarem rápido demais.
    let weight = |l: &str| (l.chars().count() as f32).max(12.0);
    let total: f32 = lines.iter().map(|l| weight(l)).sum();
    let mut t = start;
    lines
        .into_iter()
        .map(|l| {
            let at = t;
            t += (end - start) * weight(l) / total;
            (at, l.to_owned())
        })
        .collect()
}

/// Tira do título o que atrapalha a busca: "(Ao Vivo)", "[Official Video]", "- Remastered 2011", "feat. X"…
pub fn clean_title(title: &str) -> String {
    const NOISE: &[&str] = &[
        "ao vivo", "live", "remaster", "official", "oficial", "video", "vídeo", "clipe", "lyric", "letra",
        "audio", "áudio", "visualizer", "feat", "ft.", "with ", "part.", "participação", "versão", "version",
        "radio edit", "explicit", "hd", "4k", "mv",
    ];
    let mut out = String::new();
    let mut chars = title.chars().peekable();
    // Remove grupos entre () ou [] que tenham alguma palavra de "ruído".
    while let Some(c) = chars.next() {
        if c == '(' || c == '[' {
            let close = if c == '(' { ')' } else { ']' };
            let inner: String = chars.by_ref().take_while(|&x| x != close).collect();
            let low = inner.to_lowercase();
            if !NOISE.iter().any(|n| low.contains(n)) {
                out.push(c);
                out.push_str(&inner);
                out.push(close);
            }
        } else {
            out.push(c);
        }
    }
    // " - Ao Vivo", " - Remastered 2011", " feat. Fulano" no fim.
    let mut s = out.trim().to_owned();
    if let Some((head, tail)) = s.rsplit_once(" - ")
        && NOISE.iter().any(|n| tail.to_lowercase().contains(n))
    {
        s = head.trim().to_owned();
    }
    for sep in [" feat. ", " feat ", " ft. ", " Feat. ", " Ft. "] {
        if let Some((head, _)) = s.split_once(sep) {
            s = head.trim().to_owned();
        }
    }
    s
}

/// Limpa nomes de canal do YouTube ("FulanoVEVO", "Fulano - Topic", "Fulano Oficial").
fn clean_artist(artist: &str) -> String {
    let mut a = artist.trim().to_owned();
    for suffix in [" - Topic", " - Tema", "VEVO", " Oficial", " Official", " Official Channel", " Canal Oficial"] {
        if let Some(stripped) = a.strip_suffix(suffix) {
            a = stripped.trim().to_owned();
        }
    }
    a
}

/// Pares (artista, música) para tentar, do mais fiel ao mais solto.
pub fn candidates(track: &Track) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut push = |a: &str, t: &str| {
        let (a, t) = (a.trim().to_owned(), t.trim().to_owned());
        if !t.is_empty() && !out.contains(&(a.clone(), t.clone())) {
            out.push((a, t));
        }
    };
    let artist = clean_artist(&track.artist);
    let title = clean_title(&track.title);
    push(&track.artist, &track.title);
    push(&artist, &title);
    // Título no formato "Artista - Música" (vídeos do YouTube, rádios).
    if let Some((a, t)) = title.split_once(" - ") {
        push(&clean_artist(a), &clean_title(t));
    }
    // Só o primeiro artista ("A, B & C" → "A").
    let first = artist.split([',', '&', ';']).next().unwrap_or("").split(" e ").next().unwrap_or("").trim().to_owned();
    if first != artist {
        push(&first, &title);
    }
    out
}

/// Busca a letra, de preferência sincronizada. `None` se não achar (ou se for instrumental).
pub fn fetch(track: &Track, opts: &Options) -> Option<Lyrics> {
    if track.title.trim().is_empty() {
        return None;
    }
    let agent = crate::http::agent(Duration::from_secs(8));
    let tries = candidates(track);
    let mut plain: Option<Lyrics> = None;

    for (artist, title) in &tries {
        match lrclib(&agent, track, artist, title) {
            Found::Synced(lines) => return Some(Lyrics { lines, synced: true, source: "LRCLIB" }),
            Found::Plain(text) if plain.is_none() => {
                plain = Some(Lyrics { lines: approximate(&text, track.duration), synced: false, source: "LRCLIB" });
            }
            _ => {}
        }
    }
    if plain.is_none() && !opts.vagalume_key.trim().is_empty() {
        for (artist, title) in &tries {
            if let Some(text) = vagalume(&agent, &opts.vagalume_key, artist, title) {
                plain = Some(Lyrics { lines: approximate(&text, track.duration), synced: false, source: "Vagalume" });
                break;
            }
        }
    }
    plain.filter(|l| opts.plain && !l.lines.is_empty())
}

enum Found {
    Synced(Vec<(f32, String)>),
    Plain(String),
    Nothing,
}

fn lrclib(agent: &ureq::Agent, track: &Track, artist: &str, title: &str) -> Found {
    let read = |req: ureq::RequestBuilder<ureq::typestate::WithoutBody>| {
        req.call().ok().and_then(|mut r| r.body_mut().read_to_string().ok())
    };
    // 1) Busca exata (mais rápida e precisa).
    let mut req = agent.get(format!("{}/get", api())).query("track_name", title).query("artist_name", artist);
    if !track.album.is_empty() {
        req = req.query("album_name", &track.album);
    }
    if let Some(d) = track.duration {
        req = req.query("duration", (d.round() as u32).to_string());
    }
    let mut best = read(req).and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok()).map(|v| from_value(&v)).unwrap_or(Found::Nothing);
    if matches!(best, Found::Synced(_)) {
        return best;
    }
    // 2) Busca aproximada: o primeiro resultado sincronizado, senão o primeiro com letra.
    let req = agent.get(format!("{}/search", api())).query("track_name", title).query("artist_name", artist);
    if let Some(results) = read(req).and_then(|b| serde_json::from_str::<Vec<serde_json::Value>>(&b).ok()) {
        for v in &results {
            match from_value(v) {
                synced @ Found::Synced(_) => return synced,
                Found::Plain(p) if !matches!(best, Found::Plain(_)) => best = Found::Plain(p),
                _ => {}
            }
        }
    }
    best
}

fn from_value(v: &serde_json::Value) -> Found {
    if v.get("instrumental").and_then(|x| x.as_bool()) == Some(true) {
        return Found::Nothing;
    }
    if let Some(s) = v.get("syncedLyrics").and_then(|x| x.as_str()) {
        let lines = parse_lrc(s);
        if !lines.is_empty() {
            return Found::Synced(lines);
        }
    }
    match v.get("plainLyrics").and_then(|x| x.as_str()) {
        Some(p) if !p.trim().is_empty() => Found::Plain(p.to_owned()),
        _ => Found::Nothing,
    }
}

/// API oficial do Vagalume (https://api.vagalume.com.br). Só devolve a letra sem tempos.
fn vagalume(agent: &ureq::Agent, key: &str, artist: &str, title: &str) -> Option<String> {
    let base = std::env::var("HALO_VAGALUME_API").unwrap_or_else(|_| "https://api.vagalume.com.br".to_owned());
    let body = agent
        .get(format!("{base}/search.php"))
        .query("art", artist)
        .query("mus", title)
        .query("apikey", key.trim())
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    // "type": "exact" ou "aprox" quando achou; "notfound"/"song_notfound" quando não.
    if !matches!(v["type"].as_str(), Some("exact" | "aprox")) {
        return None;
    }
    let text = v["mus"].get(0)?["text"].as_str()?;
    (!text.trim().is_empty()).then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lrc() {
        let lines = parse_lrc("[00:01.50] Olá\n[ar: alguém]\n[00:03.00][00:10.00] Refrão\n[00:05.00]\n");
        assert_eq!(lines, vec![(1.5, "Olá".into()), (3.0, "Refrão".into()), (5.0, String::new()), (10.0, "Refrão".into())]);
        let l = Lyrics { lines, synced: true, source: "teste" };
        assert_eq!(l.line_at(0.5), None);
        assert_eq!(l.line_at(3.2), Some(1));
        assert_eq!(l.next_change(3.2), Some(5.0));
    }

    #[test]
    fn cleans_titles() {
        assert_eq!(clean_title("Evidências (Ao Vivo)"), "Evidências");
        assert_eq!(clean_title("Tempo Perdido - Remastered 2011"), "Tempo Perdido");
        assert_eq!(clean_title("Flowers [Official Video]"), "Flowers");
        assert_eq!(clean_title("Cheia de Manias (feat. Fulano)"), "Cheia de Manias");
        assert_eq!(clean_title("Song feat. Someone"), "Song");
        // Parênteses que fazem parte do nome ficam.
        assert_eq!(clean_title("Ainda Bem (Que Você Existe)"), "Ainda Bem (Que Você Existe)");
    }

    #[test]
    fn candidates_from_youtube_titles() {
        let t = Track {
            title: "Legião Urbana - Tempo Perdido (Clipe Oficial)".into(),
            artist: "LegiaoUrbanaVEVO".into(),
            album: String::new(),
            duration: None,
        };
        let c = candidates(&t);
        assert!(c.contains(&("Legião Urbana".into(), "Tempo Perdido".into())), "{c:?}");
    }

    #[test]
    fn approximate_spreads_lines() {
        let l = approximate("um\n\ndois\ntrês", Some(100.0));
        assert_eq!(l.len(), 3);
        assert!((l[0].0 - 8.0).abs() < 0.01);
        assert!(l[1].0 > l[0].0 && l[2].0 > l[1].0 && l[2].0 < 92.0);
    }
}
