//! Atualização automática pelas Releases do GitHub.
//!
//! Baixa o executável novo do sistema, troca pelo atual e reabre o Halo.
//! No Windows o executável em uso não pode ser apagado, mas pode ser renomeado:
//! o antigo vira `halo.exe.old` e é removido na próxima abertura.

use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// Repositório das releases.
pub const REPO: &str = "oadrianrabelo/halo";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Notas de todas as versões, embutidas no executável.
pub const CHANGELOG: &str = include_str!("../CHANGELOG.md");

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub download: Option<String>,
    pub page: String,
}

#[derive(Clone, Debug, Default)]
pub enum State {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    Downloading,
    /// Executável novo no lugar; falta só reabrir.
    Ready,
    Failed(String),
}

pub struct Updater {
    state: Arc<Mutex<State>>,
    ctx: egui::Context,
}

impl Updater {
    pub fn new(ctx: egui::Context) -> Self {
        Self { state: Arc::default(), ctx }
    }

    pub fn state(&self) -> State {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    fn spawn(&self, work: impl FnOnce(&Mutex<State>) + Send + 'static) {
        let state = self.state.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            work(&state);
            ctx.request_repaint();
        });
    }

    pub fn check(&self) {
        if matches!(self.state(), State::Checking | State::Downloading | State::Ready) {
            return;
        }
        self.set(State::Checking);
        self.spawn(|state| {
            let result = match latest() {
                Ok(r) if newer(&r.version, VERSION) => {
                    log::info!("Atualização disponível: {}", r.version);
                    State::Available(r)
                }
                Ok(r) => {
                    log::info!("Halo em dia (última versão publicada: {})", r.version);
                    State::UpToDate
                }
                Err(e) => {
                    log::warn!("Falha ao procurar atualização: {e}");
                    State::Failed(e)
                }
            };
            *state.lock().unwrap_or_else(|e| e.into_inner()) = result;
        });
    }

    pub fn install(&self, release: Release) {
        self.set(State::Downloading);
        self.spawn(move |state| {
            let result = match release.download.as_deref().ok_or("Esta versão não tem arquivo para este sistema".to_owned()).and_then(replace_self) {
                Ok(()) => {
                    log::info!("Atualizado para {}", release.version);
                    State::Ready
                }
                Err(e) => {
                    log::error!("Falha ao atualizar: {e}");
                    State::Failed(e)
                }
            };
            *state.lock().unwrap_or_else(|e| e.into_inner()) = result;
        });
    }

    fn set(&self, s: State) {
        *self.state.lock().unwrap_or_else(|e| e.into_inner()) = s;
        self.ctx.request_repaint();
    }
}

fn agent() -> ureq::Agent {
    crate::http::agent(Duration::from_secs(60))
}

/// Nome do arquivo deste sistema nas releases.
fn asset_name() -> &'static str {
    if cfg!(windows) {
        "halo-windows-x64.exe"
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") { "halo-macos-arm64" } else { "halo-macos-intel" }
    } else {
        "halo-linux-x64"
    }
}

fn latest() -> Result<Release, String> {
    let body = agent()
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let tag = v["tag_name"].as_str().ok_or("Resposta sem versão")?;
    let download = v["assets"].as_array().and_then(|assets| {
        assets
            .iter()
            .find(|a| a["name"].as_str() == Some(asset_name()))
            .and_then(|a| a["browser_download_url"].as_str())
            .map(str::to_owned)
    });
    Ok(Release {
        version: tag.trim_start_matches('v').to_owned(),
        notes: v["body"].as_str().unwrap_or_default().trim().to_owned(),
        download,
        page: v["html_url"].as_str().unwrap_or_default().to_owned(),
    })
}

/// `a` é mais nova que `b`? Compara número por número ("0.10.0" > "0.9.3").
pub fn newer(a: &str, b: &str) -> bool {
    let parse = |s: &str| s.split('.').map(|p| p.trim().parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
    parse(a) > parse(b)
}

/// Caminho do executável guardado na inicialização. Depois de renomear o arquivo em uso,
/// o sistema passa a informar o nome novo (`.old`), e é o original que precisa ser reaberto.
static EXE: OnceLock<Option<PathBuf>> = OnceLock::new();

fn exe_path() -> Result<PathBuf, String> {
    EXE.get_or_init(|| std::env::current_exe().ok()).clone().ok_or_else(|| "não sei onde está o executável".to_owned())
}

fn old_path(exe: &std::path::Path) -> PathBuf {
    let mut p = exe.as_os_str().to_owned();
    p.push(".old");
    PathBuf::from(p)
}

/// Baixa seguindo os redirecionamentos na mão: o GitHub responde 302 para o CDN, e ler o
/// corpo desse 302 falhava em algumas redes. Assim só o corpo final é lido.
fn download(url: &str) -> Result<Vec<u8>, String> {
    let agent = agent();
    let mut url = url.to_owned();
    for _ in 0..6 {
        let mut resp = agent
            .get(&url)
            .config()
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .call()
            .map_err(|e| format!("download: {e}"))?;
        let status = resp.status();
        if status.is_redirection() {
            url = resp
                .headers()
                .get("location")
                .and_then(|l| l.to_str().ok())
                .ok_or("download: redirecionamento sem destino")?
                .to_owned();
            continue;
        }
        if !status.is_success() {
            return Err(format!("download: HTTP {status}"));
        }
        return resp
            .body_mut()
            .with_config()
            .limit(100 * 1024 * 1024)
            .read_to_vec()
            .map_err(|e| format!("download: {e}"));
    }
    Err("download: redirecionamentos demais".into())
}

fn replace_self(url: &str) -> Result<(), String> {
    let bytes = download(url)?;
    if bytes.len() < 500_000 {
        return Err("O arquivo baixado parece incompleto".into());
    }
    let exe = exe_path()?;
    let old = old_path(&exe);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|e| format!("sem permissão para trocar o executável ({e})"))?;
    if let Err(e) = std::fs::write(&exe, &bytes) {
        let _ = std::fs::rename(&old, &exe); // desfaz
        return Err(format!("não consegui gravar a versão nova ({e})"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

/// Abre a versão nova (chamado ao fechar, depois de salvar as configurações).
pub fn relaunch() {
    if let Ok(exe) = exe_path() {
        match std::process::Command::new(&exe).spawn() {
            Ok(_) => log::info!("Reabrindo {}", exe.display()),
            Err(e) => log::error!("Não consegui reabrir: {e}"),
        }
    }
}

/// Apaga o executável antigo que sobrou de uma atualização.
pub fn cleanup_old_binary() {
    if let Ok(exe) = exe_path() {
        let _ = std::fs::remove_file(old_path(&exe));
    }
}

/// Abre um link ou pasta no app padrão do sistema.
pub fn open(target: &str) {
    let cmd = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(cmd).arg(target).spawn() {
        log::warn!("Não consegui abrir {target}: {e}");
    }
}

/// Seções do CHANGELOG.md: (título, linhas).
pub fn changelog() -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for line in CHANGELOG.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            out.push((title.trim().to_owned(), Vec::new()));
        } else if let Some((_, lines)) = out.last_mut()
            && !line.trim().is_empty()
        {
            lines.push(line.trim().to_owned());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(newer("0.10.0", "0.9.3"));
        assert!(newer("1.0.0", "0.99.99"));
        assert!(!newer("0.4.0", "0.4.0"));
        assert!(!newer("0.3.9", "0.4.0"));
    }

    #[test]
    fn changelog_has_current_version() {
        let log = changelog();
        assert!(log.first().is_some_and(|(t, _)| t.starts_with(VERSION)), "CHANGELOG.md precisa começar pela versão {VERSION}");
    }
}
