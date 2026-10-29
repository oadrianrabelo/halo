//! Grava todos os sons do Halo em uma pasta, para ouvir: `cargo run --example dump_sounds -- pasta`
#[path = "../src/sound.rs"]
#[allow(dead_code)]
mod sound;

// sound.rs usa crate::logging só para o cache; aqui não é chamado.
#[allow(dead_code)]
mod logging {
    pub fn data_dir() -> std::path::PathBuf {
        std::env::temp_dir()
    }
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "sons".into());
    std::fs::create_dir_all(&dir).unwrap();
    for s in sound::Sound::ALL {
        let samples = sound::synth(s);
        if samples.is_empty() {
            continue;
        }
        let path = format!("{dir}/{}.wav", s.label().to_lowercase().replace(' ', "-"));
        std::fs::write(&path, sound::wav(&samples, 0.8)).unwrap();
        println!("{path}");
    }
}
