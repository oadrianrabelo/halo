//! "Tocando agora": lê a música do sistema e busca a letra, numa thread separada.
//!
//! - Windows: controles de mídia do sistema (os mesmos do painel de volume).
//! - Linux: MPRIS pelo D-Bus (Spotify, navegadores, VLC, Rhythmbox…).
//! - macOS: AppleScript para Spotify e Música.

use crate::art::{self, Art};
use crate::lyrics::{self, Lyrics};
use eframe::egui;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Duração em segundos, se o player informar.
    pub duration: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct Playback {
    pub track: Track,
    pub playing: bool,
    /// Posição em segundos no instante `at`.
    pub position: f32,
    pub at: Instant,
}

impl Playback {
    /// Posição estimada agora, contando o tempo desde a última leitura.
    pub fn position_now(&self, now: Instant) -> f32 {
        if self.playing { self.position + (now - self.at).as_secs_f32() } else { self.position }
    }
}

#[derive(Clone, Copy)]
pub enum Command {
    PlayPause,
    Next,
    Previous,
}

#[derive(Default, Clone)]
pub enum LyricsState {
    #[default]
    None,
    Loading,
    Found(Arc<Lyrics>),
    NotFound,
}

#[derive(Default)]
pub struct Shared {
    pub playback: Option<Playback>,
    pub lyrics: LyricsState,
    pub art: Option<Arc<Art>>,
}

/// Thread de leitura. Para sozinha quando este valor é descartado.
pub struct Media {
    pub shared: Arc<Mutex<Shared>>,
    commands: Sender<Command>,
}

impl Media {
    /// `lyrics: None` desliga a busca de letras.
    pub fn start(ctx: egui::Context, lyrics: Option<lyrics::Options>) -> Self {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let (tx, rx) = mpsc::channel();
        let state = shared.clone();
        thread::Builder::new()
            .name("halo-media".into())
            .spawn(move || run(ctx, state, rx, lyrics))
            .expect("thread de mídia");
        Self { shared, commands: tx }
    }

    pub fn send(&self, cmd: Command) {
        let _ = self.commands.send(cmd);
    }

    /// Capa da música atual, se houver.
    pub fn art(&self) -> Option<Arc<Art>> {
        self.shared.lock().ok()?.art.clone()
    }

    pub fn snapshot(&self) -> (Option<Playback>, LyricsState) {
        let s = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        (s.playback.clone(), s.lyrics.clone())
    }
}

fn run(ctx: egui::Context, shared: Arc<Mutex<Shared>>, commands: Receiver<Command>, opts: Option<lyrics::Options>) {
    let mut backend = platform::Backend::new();
    let mut cache: HashMap<String, Option<Arc<Lyrics>>> = HashMap::new();
    let mut last_track: Option<Track> = None;
    let mut last_playing = false;
    let mut art_cache: HashMap<String, Arc<Art>> = HashMap::new();
    // Alguns players só entregam a capa um instante depois de trocar de música: tenta mais algumas vezes.
    let mut art_tries = 0;

    loop {
        let playback = backend.query();
        let track = playback.as_ref().map(|p| p.track.clone());
        let playing = playback.as_ref().is_some_and(|p| p.playing);
        let changed = track != last_track || playing != last_playing;

        if track != last_track {
            set(&shared, |s| s.art = None);
            art_tries = 4;
        }
        if let Some(t) = &track
            && art_tries > 0
        {
            art_tries -= 1;
            let key = format!("{}\u{1}{}\u{1}{}", t.artist, t.album, t.title);
            let found = art_cache.get(&key).cloned().or_else(|| {
                let a = backend.art().and_then(art::load).map(Arc::new)?;
                if art_cache.len() > 40 {
                    art_cache.clear();
                }
                art_cache.insert(key, a.clone());
                Some(a)
            });
            if found.is_some() {
                art_tries = 0;
                set(&shared, |s| s.art = found);
                ctx.request_repaint();
            }
        }

        let lyrics = if track != last_track {
            match &track {
                Some(t) if opts.is_some() => {
                    let key = format!("{}\u{1}{}", t.artist, t.title);
                    match cache.get(&key) {
                        Some(found) => to_state(found),
                        None => {
                            set(&shared, |s| {
                                s.playback = playback.clone();
                                s.lyrics = LyricsState::Loading;
                            });
                            ctx.request_repaint();
                            let found = opts.as_ref().and_then(|o| lyrics::fetch(t, o)).map(Arc::new);
                            match &found {
                                Some(l) => log::info!("Letra de \"{}\": {} ({})", t.title, l.source, if l.synced { "sincronizada" } else { "aproximada" }),
                                None => log::info!("Sem letra para \"{}\" — {}", t.title, t.artist),
                            }
                            let state = to_state(&found);
                            cache.insert(key, found);
                            state
                        }
                    }
                }
                _ => LyricsState::None,
            }
        } else {
            shared.lock().map(|s| s.lyrics.clone()).unwrap_or_default()
        };

        set(&shared, |s| {
            s.playback = playback;
            s.lyrics = lyrics;
        });
        if changed {
            ctx.request_repaint();
        }
        last_track = track;
        last_playing = playing;

        // Espera 1 s, mas responde na hora a um comando (play/pause, próxima…).
        match commands.recv_timeout(Duration::from_secs(1)) {
            Ok(cmd) => {
                backend.send(cmd);
                thread::sleep(Duration::from_millis(150));
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn to_state(found: &Option<Arc<Lyrics>>) -> LyricsState {
    match found {
        Some(l) => LyricsState::Found(l.clone()),
        None => LyricsState::NotFound,
    }
}

fn set(shared: &Mutex<Shared>, f: impl FnOnce(&mut Shared)) {
    let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut s);
}

#[cfg(windows)]
mod platform {
    use super::{Command, Playback, Track};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
    };

    pub struct Backend {
        manager: Option<Manager>,
    }

    impl Backend {
        pub fn new() -> Self {
            Self { manager: Manager::RequestAsync().and_then(|op| op.join()).ok() }
        }

        fn session(&self) -> Option<Session> {
            self.manager.as_ref()?.GetCurrentSession().ok()
        }

        pub fn query(&mut self) -> Option<Playback> {
            let session = self.session()?;
            let props = session.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
            let title = props.Title().ok()?.to_string();
            if title.is_empty() {
                return None;
            }
            let playing = session.GetPlaybackInfo().ok()?.PlaybackStatus().ok()? == Status::Playing;
            let timeline = session.GetTimelineProperties().ok()?;
            let ticks = |t: i64| t as f32 / 10_000_000.0;
            let end = timeline.EndTime().map(|t| ticks(t.Duration)).unwrap_or(0.0);
            let mut position = timeline.Position().map(|t| ticks(t.Duration)).unwrap_or(0.0);
            // A posição é de quando o player atualizou pela última vez; avança até agora.
            if playing && let Ok(updated) = timeline.LastUpdatedTime() {
                let now_ticks = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as i64 / 100).unwrap_or(0)
                    + 116_444_736_000_000_000;
                let elapsed = ticks(now_ticks - updated.UniversalTime);
                if (0.0..3600.0).contains(&elapsed) {
                    position += elapsed;
                }
            }
            Some(Playback {
                track: Track {
                    title,
                    artist: props.Artist().map(|s| s.to_string()).unwrap_or_default(),
                    album: props.AlbumTitle().map(|s| s.to_string()).unwrap_or_default(),
                    duration: (end > 0.0).then_some(end),
                },
                playing,
                position,
                at: Instant::now(),
            })
        }

        pub fn art(&mut self) -> Option<crate::art::Source> {
            use windows::Storage::Streams::DataReader;
            let props = self.session()?.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
            let stream = props.Thumbnail().ok()?.OpenReadAsync().ok()?.join().ok()?;
            let size = stream.Size().ok()? as u32;
            if size == 0 || size > 10 * 1024 * 1024 {
                return None;
            }
            let reader = DataReader::CreateDataReader(&stream).ok()?;
            reader.LoadAsync(size).ok()?.join().ok()?;
            let mut buf = vec![0u8; size as usize];
            reader.ReadBytes(&mut buf).ok()?;
            Some(crate::art::Source::Bytes(buf))
        }

        pub fn send(&mut self, cmd: Command) {
            let Some(s) = self.session() else { return };
            let op = match cmd {
                Command::PlayPause => s.TryTogglePlayPauseAsync(),
                Command::Next => s.TrySkipNextAsync(),
                Command::Previous => s.TrySkipPreviousAsync(),
            };
            let _ = op.and_then(|op| op.join());
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod platform {
    use super::{Command, Playback, Track};
    use std::collections::HashMap;
    use std::time::Instant;
    use zbus::blocking::{Connection, Proxy, fdo::DBusProxy};
    use zbus::proxy::CacheProperties;
    use zbus::zvariant::OwnedValue;

    const PREFIX: &str = "org.mpris.MediaPlayer2.";

    pub struct Backend {
        conn: Option<Connection>,
        /// Último player usado, para os comandos irem para o mesmo.
        current: Option<String>,
        art_url: Option<String>,
    }

    impl Backend {
        pub fn new() -> Self {
            Self { conn: Connection::session().ok(), current: None, art_url: None }
        }

        fn player(&self, name: &str) -> Option<Proxy<'static>> {
            let conn = self.conn.as_ref()?;
            zbus::blocking::proxy::Builder::new(conn)
                .destination(name.to_owned())
                .ok()?
                .path("/org/mpris/MediaPlayer2")
                .ok()?
                .interface("org.mpris.MediaPlayer2.Player")
                .ok()?
                .cache_properties(CacheProperties::No)
                .build()
                .ok()
        }

        pub fn query(&mut self) -> Option<Playback> {
            let conn = self.conn.as_ref()?;
            let names = DBusProxy::new(conn).ok()?.list_names().ok()?;
            let mut best: Option<(String, Proxy<'static>, String)> = None;
            for name in names.iter().map(|n| n.as_str()).filter(|n| n.starts_with(PREFIX)) {
                let Some(p) = self.player(name) else { continue };
                let status: String = p.get_property("PlaybackStatus").unwrap_or_default();
                let better = match &best {
                    None => status != "Stopped",
                    // Quem está tocando ganha de quem está pausado.
                    Some((_, _, s)) => status == "Playing" && s != "Playing",
                };
                if better {
                    best = Some((name.to_owned(), p, status));
                }
            }
            let (name, p, status) = best?;
            let meta: HashMap<String, OwnedValue> = p.get_property("Metadata").ok()?;
            let text = |k: &str| meta.get(k).and_then(|v| String::try_from(v.clone()).ok()).unwrap_or_default();
            let title = text("xesam:title");
            if title.is_empty() {
                return None;
            }
            let artist = meta
                .get("xesam:artist")
                .and_then(|v| Vec::<String>::try_from(v.clone()).ok())
                .map(|a| a.join(", "))
                .unwrap_or_default();
            let micros = |v: &OwnedValue| {
                i64::try_from(v.clone()).ok().or_else(|| u64::try_from(v.clone()).ok().map(|x| x as i64))
            };
            let duration = meta.get("mpris:length").and_then(micros).map(|us| us as f32 / 1e6);
            let position = p.get_property::<i64>("Position").map(|us| us as f32 / 1e6).unwrap_or(0.0);
            self.current = Some(name);
            self.art_url = Some(text("mpris:artUrl")).filter(|u| !u.is_empty());
            Some(Playback {
                track: Track { title, artist, album: text("xesam:album"), duration },
                playing: status == "Playing",
                position,
                at: Instant::now(),
            })
        }

        pub fn art(&mut self) -> Option<crate::art::Source> {
            self.art_url.clone().map(crate::art::Source::Url)
        }

        pub fn send(&mut self, cmd: Command) {
            let Some(p) = self.current.clone().and_then(|n| self.player(&n)) else { return };
            let method = match cmd {
                Command::PlayPause => "PlayPause",
                Command::Next => "Next",
                Command::Previous => "Previous",
            };
            let _ = p.call_method(method, &());
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{Command, Playback, Track};
    use std::process::Command as Process;
    use std::time::Instant;

    /// Consulta o app pelo AppleScript dele. Só é chamado se o app já estiver aberto,
    /// para não abrir nada nem perguntar onde está um app que não existe.
    fn script(app: &str) -> String {
        format!(
            r#"tell application "{app}"
    if player state is stopped then return ""
    set t to current track
    return (player state as text) & tab & (name of t) & tab & (artist of t) & tab & (album of t) & tab & (duration of t) & tab & player position{art}
end tell"#,
            // Só o Spotify informa a capa como link.
            art = if app == "Spotify" { " & tab & (artwork url of t)" } else { "" }
        )
    }

    fn running(app: &str) -> bool {
        Process::new("pgrep").args(["-xq", app]).status().is_ok_and(|s| s.success())
    }

    pub struct Backend {
        app: Option<String>,
        art_url: Option<String>,
    }

    impl Backend {
        pub fn new() -> Self {
            Self { app: None, art_url: None }
        }

        pub fn query(&mut self) -> Option<Playback> {
            let app = ["Spotify", "Music"].into_iter().find(|a| running(a))?;
            let out = Process::new("osascript").args(["-e", &script(app)]).output().ok()?;
            let line = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 6 {
                return None;
            }
            let num = |s: &str| s.replace(',', ".").parse::<f32>().ok();
            let mut duration = num(f[4]);
            // O Spotify informa a duração em milissegundos; o Música, em segundos.
            if app == "Spotify" {
                duration = duration.map(|d| d / 1000.0);
            }
            self.app = Some(app.to_owned());
            self.art_url = f.get(6).map(|u| u.trim().to_owned()).filter(|u| u.starts_with("http"));
            Some(Playback {
                track: Track { title: f[1].to_owned(), artist: f[2].to_owned(), album: f[3].to_owned(), duration },
                playing: f[0] == "playing",
                position: num(f[5]).unwrap_or(0.0),
                at: Instant::now(),
            })
        }

        pub fn art(&mut self) -> Option<crate::art::Source> {
            self.art_url.clone().map(crate::art::Source::Url)
        }

        pub fn send(&mut self, cmd: Command) {
            let Some(app) = &self.app else { return };
            let action = match cmd {
                Command::PlayPause => "playpause",
                Command::Next => "next track",
                Command::Previous => "previous track",
            };
            let _ = Process::new("osascript").args(["-e", &format!("tell application \"{app}\" to {action}")]).output();
        }
    }
}
