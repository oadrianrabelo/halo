//! Carrega a fonte escolhida. A fonte do sistema é lida do disco na hora,
//! então o executável não precisa carregar nenhuma.

use crate::theme::FontChoice;
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

/// Aplica a fonte. Retorna uma mensagem de erro se o arquivo escolhido não abriu.
pub fn apply(ctx: &egui::Context, choice: &FontChoice, thin_digits: bool) -> Result<(), String> {
    let mut defs = FontDefinitions::default();
    let builtin = defs.families[&FontFamily::Proportional].clone();
    let mut error = None;

    let (regular, light) = match choice {
        FontChoice::Builtin => (None, None),
        FontChoice::System => system_fonts(),
        FontChoice::Custom(path) => match std::fs::read(path) {
            Ok(bytes) => (Some(bytes), None),
            Err(e) => {
                error = Some(format!("Não consegui abrir a fonte: {e}"));
                (None, None)
            }
        },
    };

    let mut clock_family = builtin.clone();
    if let Some(bytes) = regular {
        defs.font_data.insert("ui".into(), Arc::new(FontData::from_owned(bytes)));
        prepend(&mut defs, FontFamily::Proportional, "ui");
        clock_family.insert(0, "ui".into());
    }
    if let (Some(bytes), true) = (light, thin_digits) {
        defs.font_data.insert("ui-light".into(), Arc::new(FontData::from_owned(bytes)));
        clock_family.insert(0, "ui-light".into());
    }
    defs.families.insert(clock(), clock_family);
    ctx.set_fonts(defs);

    error.map_or(Ok(()), Err)
}

fn prepend(defs: &mut FontDefinitions, family: FontFamily, name: &str) {
    defs.families.entry(family).or_default().insert(0, name.into());
}

/// (regular, light) da fonte de interface do sistema.
fn system_fonts() -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    #[cfg(windows)]
    {
        let dir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        let font = |f: &str| std::fs::read(Path::new(&dir).join("Fonts").join(f)).ok();
        (font("segoeui.ttf"), font("segoeuil.ttf"))
    }

    #[cfg(target_os = "macos")]
    {
        let font = |f: &str| std::fs::read(f).ok();
        let regular = font("/System/Library/Fonts/SFNS.ttf")
            .or_else(|| font("/System/Library/Fonts/Helvetica.ttc"));
        (regular, None)
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Em ordem de preferência: (regular, light).
        const WANTED: &[(&str, &str)] = &[
            ("Inter-Regular.ttf", "Inter-Light.ttf"),
            ("Inter-Regular.otf", "Inter-Light.otf"),
            ("Cantarell-Regular.otf", "Cantarell-Light.otf"),
            ("Ubuntu-R.ttf", "Ubuntu-L.ttf"),
            ("NotoSans-Regular.ttf", "NotoSans-Light.ttf"),
            ("OpenSans-Regular.ttf", "OpenSans-Light.ttf"),
            ("Roboto-Regular.ttf", "Roboto-Light.ttf"),
            ("DejaVuSans.ttf", "DejaVuSans-ExtraLight.ttf"),
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
        let find = |name: &str| found.iter().find(|p| p.file_name().is_some_and(|f| f == name));
        for (regular, light) in WANTED {
            if let Some(r) = find(regular) {
                return (std::fs::read(r).ok(), find(light).and_then(|l| std::fs::read(l).ok()));
            }
        }
        (None, None)
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
