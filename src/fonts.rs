//! Carrega a fonte escolhida. A fonte do sistema é lida do disco na hora,
//! então o executável não precisa carregar nenhuma.

use crate::theme::{DigitWeight, FontChoice};
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
#[cfg(not(target_os = "macos"))]
use std::path::Path;
#[cfg(all(unix, not(target_os = "macos")))]
use std::path::PathBuf;
use std::sync::Arc;

/// Família usada nos números do relógio.
pub fn clock() -> FontFamily {
    FontFamily::Name("clock".into())
}

/// Arquivos de uma família: regular e, se existirem, as variações fina e forte.
#[derive(Default)]
struct Faces {
    regular: Option<Vec<u8>>,
    light: Option<Vec<u8>>,
    bold: Option<Vec<u8>>,
}

/// Aplica a fonte. Retorna uma mensagem de erro se o arquivo escolhido não abriu.
pub fn apply(ctx: &egui::Context, choice: &FontChoice, weight: DigitWeight) -> Result<(), String> {
    let mut defs = FontDefinitions::default();
    let builtin = defs.families[&FontFamily::Proportional].clone();
    let mut error = None;

    let faces = match choice {
        FontChoice::Builtin => Faces::default(),
        // Inter recortada para o alfabeto latino (~50 KB por peso). Licença em assets/fonts.
        FontChoice::Inter => Faces {
            regular: Some(include_bytes!("../assets/fonts/Inter-Regular.ttf").to_vec()),
            light: Some(include_bytes!("../assets/fonts/Inter-Light.ttf").to_vec()),
            bold: Some(include_bytes!("../assets/fonts/Inter-SemiBold.ttf").to_vec()),
        },
        FontChoice::System => system_fonts(),
        FontChoice::Custom(path) => match std::fs::read(path) {
            Ok(bytes) => Faces { regular: Some(bytes), ..Default::default() },
            Err(e) => {
                error = Some(format!("Não consegui abrir a fonte: {e}"));
                Faces::default()
            }
        },
    };

    let mut clock_family = builtin.clone();
    if let Some(bytes) = faces.regular {
        defs.font_data.insert("ui".into(), Arc::new(FontData::from_owned(bytes)));
        prepend(&mut defs, FontFamily::Proportional, "ui");
        clock_family.insert(0, "ui".into());
    }
    let variant = match weight {
        DigitWeight::Thin => faces.light,
        DigitWeight::Regular => None,
        DigitWeight::Bold => faces.bold,
    };
    if let Some(bytes) = variant {
        defs.font_data.insert("clock".into(), Arc::new(FontData::from_owned(bytes)));
        clock_family.insert(0, "clock".into());
    }
    defs.families.insert(clock(), clock_family);
    ctx.set_fonts(defs);

    error.map_or(Ok(()), Err)
}

fn prepend(defs: &mut FontDefinitions, family: FontFamily, name: &str) {
    defs.families.entry(family).or_default().insert(0, name.into());
}

/// Fonte de interface do sistema, com as variações que existirem.
fn system_fonts() -> Faces {
    #[cfg(windows)]
    {
        let dir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        let font = |f: &str| std::fs::read(Path::new(&dir).join("Fonts").join(f)).ok();
        Faces { regular: font("segoeui.ttf"), light: font("segoeuil.ttf"), bold: font("seguisb.ttf") }
    }

    #[cfg(target_os = "macos")]
    {
        let font = |f: &str| std::fs::read(f).ok();
        let regular = font("/System/Library/Fonts/SFNS.ttf").or_else(|| font("/System/Library/Fonts/Helvetica.ttc"));
        Faces { regular, ..Default::default() }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Em ordem de preferência: (regular, fina, forte).
        const WANTED: &[(&str, &str, &str)] = &[
            ("Inter-Regular.ttf", "Inter-Light.ttf", "Inter-SemiBold.ttf"),
            ("Inter-Regular.otf", "Inter-Light.otf", "Inter-SemiBold.otf"),
            ("Cantarell-Regular.otf", "Cantarell-Light.otf", "Cantarell-Bold.otf"),
            ("Ubuntu-R.ttf", "Ubuntu-L.ttf", "Ubuntu-M.ttf"),
            ("NotoSans-Regular.ttf", "NotoSans-Light.ttf", "NotoSans-SemiBold.ttf"),
            ("OpenSans-Regular.ttf", "OpenSans-Light.ttf", "OpenSans-SemiBold.ttf"),
            ("Roboto-Regular.ttf", "Roboto-Light.ttf", "Roboto-Medium.ttf"),
            ("DejaVuSans.ttf", "DejaVuSans-ExtraLight.ttf", "DejaVuSans-Bold.ttf"),
        ];
        let mut found = Vec::new();
        let mut dirs = vec![PathBuf::from("/usr/share/fonts"), PathBuf::from("/usr/local/share/fonts")];
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(Path::new(&home).join(".local/share/fonts"));
            dirs.push(Path::new(&home).join(".fonts"));
        }
        for d in dirs {
            collect(&d, 5, &mut found);
        }
        let read = |name: &str| {
            found.iter().find(|p| p.file_name().is_some_and(|f| f == name)).and_then(|p| std::fs::read(p).ok())
        };
        for (regular, light, bold) in WANTED {
            if let Some(r) = read(regular) {
                return Faces { regular: Some(r), light: read(light), bold: read(bold) };
            }
        }
        Faces::default()
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn collect(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            if depth > 0 {
                collect(&path, depth - 1, out);
            }
        } else {
            out.push(path);
        }
    }
}
