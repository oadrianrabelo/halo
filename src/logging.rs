//! Log em arquivo e captura de travamentos.
//!
//! Windows: %LOCALAPPDATA%\Halo\halo.log
//! macOS:   ~/Library/Application Support/Halo/halo.log
//! Linux:   ~/.local/share/halo/halo.log

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Pasta de dados do Halo (log, marcadores de inicialização).
pub fn data_dir() -> PathBuf {
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).map(|p| p.join("Halo"));
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support/Halo"));
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .map(|p| p.join("halo"));
    base.unwrap_or_else(std::env::temp_dir)
}

pub fn log_path() -> PathBuf {
    data_dir().join("halo.log")
}

struct FileLogger {
    file: Mutex<File>,
}

impl log::Log for FileLogger {
    fn enabled(&self, m: &log::Metadata<'_>) -> bool {
        // Tudo do Halo a partir de info; das bibliotecas, só avisos e erros.
        if m.target().starts_with("halo") { m.level() <= log::Level::Info } else { m.level() <= log::Level::Warn }
    }

    fn log(&self, r: &log::Record<'_>) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let line = format!("{} {:<5} {}: {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), r.level(), r.target(), r.args());
        if let Ok(mut f) = self.file.lock() {
            let _ = f.write_all(line.as_bytes());
        }
        #[cfg(debug_assertions)]
        eprint!("{line}");
    }

    fn flush(&self) {
        if let Ok(mut f) = self.file.lock() {
            let _ = f.flush();
        }
    }
}

pub fn init() {
    let dir = data_dir();
    let _ = fs::create_dir_all(&dir);
    let path = log_path();
    // Mantém o log pequeno: passou de 1 MB, vira halo.old.log.
    if fs::metadata(&path).is_ok_and(|m| m.len() > 1_000_000) {
        let _ = fs::rename(&path, dir.join("halo.old.log"));
    }
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
        let logger = FileLogger { file: Mutex::new(file) };
        if log::set_boxed_logger(Box::new(logger)).is_ok() {
            log::set_max_level(log::LevelFilter::Info);
        }
    }

    std::panic::set_hook(Box::new(|info| {
        let bt = std::backtrace::Backtrace::force_capture();
        log::error!("TRAVOU: {info}\n{bt}");
        log::logger().flush();
        fatal(&format!("O Halo encontrou um erro e precisa fechar.\n\n{info}"));
    }));

    log::info!(
        "Halo {} iniciando ({} {})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}

/// Mostra uma mensagem de erro que a pessoa consegue ver mesmo sem console.
pub fn fatal(msg: &str) {
    let full = format!("{msg}\n\nDetalhes no log:\n{}", log_path().display());
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let (text, title) = (wide(&full), wide("Halo"));
        unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR) };
    }
    #[cfg(not(windows))]
    eprintln!("{full}");
}
