//! Sons de aviso. Os sons do Halo são sintetizados aqui mesmo (sem arquivos de áudio no executável),
//! gravados uma vez como .wav na pasta de dados e tocados pelo próprio sistema — sem biblioteca de áudio.

use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sound {
    Sino,
    Gota,
    Madeira,
    Cristal,
    Acorde,
    Digital,
    Tigela,
    /// Som padrão do sistema.
    Sistema,
    /// Arquivo escolhido pela pessoa.
    Arquivo,
}

impl Sound {
    pub const ALL: [Sound; 9] = [
        Sound::Sino,
        Sound::Gota,
        Sound::Madeira,
        Sound::Cristal,
        Sound::Acorde,
        Sound::Digital,
        Sound::Tigela,
        Sound::Sistema,
        Sound::Arquivo,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Sound::Sino => "Sino",
            Sound::Gota => "Gota",
            Sound::Madeira => "Madeira",
            Sound::Cristal => "Cristal",
            Sound::Acorde => "Acorde",
            Sound::Digital => "Digital",
            Sound::Tigela => "Tigela tibetana",
            Sound::Sistema => "Do sistema",
            Sound::Arquivo => "Arquivo…",
        }
    }
}

const RATE: u32 = 44_100;

/// Toca `sound` no volume dado (0 a 1). `file` só é usado com `Sound::Arquivo`.
pub fn play(sound: Sound, volume: f32, file: &str) {
    match sound {
        Sound::Sistema => system_default(),
        Sound::Arquivo if !file.trim().is_empty() => play_file(file.trim()),
        Sound::Arquivo => system_default(),
        _ => match cached_wav(sound, volume) {
            Some(path) => play_file(&path.to_string_lossy()),
            None => system_default(),
        },
    }
}

/// Gera (só na primeira vez) o .wav do som nesse volume.
fn cached_wav(sound: Sound, volume: f32) -> Option<PathBuf> {
    let volume = volume.clamp(0.0, 1.0);
    let dir = crate::logging::data_dir().join("sons");
    let path = dir.join(format!("{sound:?}-{}.wav", (volume * 100.0).round() as u32).to_lowercase());
    if !path.exists() {
        std::fs::create_dir_all(&dir).ok()?;
        std::fs::write(&path, wav(&synth(sound), volume)).ok()?;
    }
    Some(path)
}

/// Amostras (-1 a 1) de cada som.
pub fn synth(sound: Sound) -> Vec<f32> {
    let secs = match sound {
        Sound::Sino => 2.4,
        Sound::Gota => 0.9,
        Sound::Madeira => 0.6,
        Sound::Cristal => 1.8,
        Sound::Acorde => 2.2,
        Sound::Digital => 0.6,
        Sound::Tigela => 4.5,
        Sound::Sistema | Sound::Arquivo => 0.0,
    };
    let n = (secs * RATE as f32) as usize;
    let mut out = vec![0.0f32; n];
    // Soma de senoides com decaimento exponencial, começando em `start` segundos.
    let mut tone = |start: f32, freq: f32, amp: f32, decay: f32| {
        let s0 = (start * RATE as f32) as usize;
        for (i, v) in out.iter_mut().enumerate().skip(s0) {
            let t = (i - s0) as f32 / RATE as f32;
            let attack = (t / 0.004).min(1.0); // 4 ms de ataque para não estalar
            *v += amp * attack * (-t * decay).exp() * (TAU * freq * t).sin();
        }
    };
    match sound {
        Sound::Sino => {
            // Parciais de sino (não harmônicos): os agudos somem antes.
            let f = 660.0;
            for (ratio, amp) in [(0.5, 0.35), (1.0, 1.0), (2.0, 0.55), (2.76, 0.4), (5.4, 0.22), (8.93, 0.1)] {
                tone(0.0, f * ratio, amp, 1.4 * f32::sqrt(ratio));
            }
        }
        Sound::Gota => {
            // Duas gotas: senoide que cai rápido de tom.
            for (start, from, to, amp) in [(0.0, 1500.0, 650.0, 1.0), (0.32, 1250.0, 560.0, 0.55)] {
                let s0 = (start * RATE as f32) as usize;
                let mut phase = 0.0f32;
                for (i, v) in out.iter_mut().enumerate().skip(s0) {
                    let t = (i - s0) as f32 / RATE as f32;
                    let freq = to + (from - to) * (-t / 0.03).exp();
                    phase += TAU * freq / RATE as f32;
                    *v += amp * (-t * 9.0).exp() * phase.sin();
                }
            }
        }
        Sound::Madeira => {
            // Duas batidas curtas de bloco de madeira.
            for (start, amp) in [(0.0, 1.0), (0.17, 0.8)] {
                tone(start, 820.0, amp, 38.0);
                tone(start, 1640.0, amp * 0.35, 60.0);
                tone(start, 2460.0, amp * 0.15, 90.0);
            }
        }
        Sound::Cristal => {
            // Arpejo de taças: Mi, Si, Mi agudo.
            for (i, f) in [1318.5, 1975.5, 2637.0].into_iter().enumerate() {
                let start = i as f32 * 0.12;
                tone(start, f, 0.8, 3.0);
                tone(start, f * 2.01, 0.25, 5.0);
                tone(start, f * 3.02, 0.1, 7.0);
            }
        }
        Sound::Acorde => {
            // Dó maior dedilhado, suave.
            for (i, f) in [523.25, 659.25, 783.99, 1046.5].into_iter().enumerate() {
                let start = i as f32 * 0.07;
                tone(start, f, 0.7, 1.9);
                tone(start, f * 2.0, 0.18, 3.0);
            }
        }
        Sound::Digital => {
            // Três bipes curtos, com bordas suaves.
            for k in 0..3 {
                let s0 = ((k as f32 * 0.16) * RATE as f32) as usize;
                let len = (0.08 * RATE as f32) as usize;
                for i in 0..len.min(n.saturating_sub(s0)) {
                    let t = i as f32 / RATE as f32;
                    let edge = (i.min(len - i) as f32 / (0.006 * RATE as f32)).min(1.0);
                    let x = (TAU * 1046.5 * t).sin();
                    out[s0 + i] += 0.6 * edge * (x + 0.25 * (3.0 * TAU * 1046.5 * t).sin());
                }
            }
        }
        Sound::Tigela => {
            // Duas frequências quase iguais fazem a "ondulação" típica da tigela.
            let f = 220.0;
            tone(0.0, f, 0.8, 0.7);
            tone(0.0, f * 1.007, 0.6, 0.7);
            tone(0.0, f * 2.71, 0.35, 1.1);
            tone(0.0, f * 5.1, 0.12, 1.8);
        }
        Sound::Sistema | Sound::Arquivo => {}
    }
    // Normaliza o pico e suaviza o final.
    let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1e-6);
    let fade = (0.05 * RATE as f32) as usize;
    let len = out.len();
    for (i, v) in out.iter_mut().enumerate() {
        *v /= peak;
        if i + fade > len {
            *v *= (len - i) as f32 / fade as f32;
        }
    }
    out
}

/// WAV mono 16 bits.
pub fn wav(samples: &[f32], volume: f32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    let gain = 0.9 * volume * i16::MAX as f32;
    for s in samples {
        out.extend_from_slice(&((s * gain).round() as i16).to_le_bytes());
    }
    out
}

fn play_file(file: &str) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_FILENAME, SND_NODEFAULT};
        let wide: Vec<u16> = file.encode_utf16().chain(std::iter::once(0)).collect();
        let ok = unsafe { PlaySoundW(wide.as_ptr(), std::ptr::null_mut(), SND_FILENAME | SND_ASYNC | SND_NODEFAULT) != 0 };
        if !ok {
            log::warn!("Não consegui tocar {file}");
            system_default();
        }
    }

    #[cfg(target_os = "macos")]
    spawn("afplay", &[file]);

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Tenta os players mais comuns; se nenhum existir, fica só o aviso visual.
        if !(spawn("paplay", &[file]) || spawn("pw-play", &[file]) || spawn("aplay", &["-q", file])) {
            log::warn!("Nenhum player de áudio encontrado (paplay, pw-play ou aplay)");
        }
    }
}

fn system_default() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Diagnostics::Debug::MessageBeep;
        use windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONASTERISK;
        unsafe { MessageBeep(MB_ICONASTERISK) };
    }
    #[cfg(target_os = "macos")]
    spawn("afplay", &["/System/Library/Sounds/Glass.aiff"]);
    #[cfg(all(unix, not(target_os = "macos")))]
    play_file("/usr/share/sounds/freedesktop/stereo/complete.oga");
}

#[cfg(unix)]
fn spawn(cmd: &str, args: &[&str]) -> bool {
    use std::process::{Command, Stdio};
    Command::new(cmd).args(args).stdout(Stdio::null()).stderr(Stdio::null()).spawn().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_is_audible_and_clean() {
        for s in Sound::ALL {
            let samples = synth(s);
            if matches!(s, Sound::Sistema | Sound::Arquivo) {
                assert!(samples.is_empty());
                continue;
            }
            let peak = samples.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!((0.99..=1.0001).contains(&peak), "{s:?} pico {peak}");
            assert!(samples.last().unwrap().abs() < 0.01, "{s:?} termina com estalo");
            let w = wav(&samples, 0.5);
            assert_eq!(&w[0..4], b"RIFF");
            assert_eq!(w.len(), 44 + samples.len() * 2);
        }
    }
}
