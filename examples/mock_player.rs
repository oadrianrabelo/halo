//! Player MPRIS falso para testar o painel de música no Linux:
//! `dbus-run-session -- sh -c 'cargo run --example mock_player & cargo run'`
#[cfg(all(unix, not(target_os = "macos")))]
mod mpris {
    use std::collections::HashMap;
    use std::time::Instant;
    use zbus::zvariant::{OwnedValue, Value};

    struct Player {
        start: Instant,
    }

    #[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
    impl Player {
        #[zbus(property)]
        fn playback_status(&self) -> String {
            "Playing".into()
        }
        #[zbus(property)]
        fn metadata(&self) -> HashMap<String, OwnedValue> {
            let v = |x: Value<'_>| x.try_to_owned().unwrap();
            HashMap::from([
                ("xesam:title".into(), v(Value::from("Canção de Teste"))),
                ("xesam:artist".into(), v(Value::from(vec!["Artista Exemplo"]))),
                ("xesam:album".into(), v(Value::from("Álbum"))),
                ("mpris:length".into(), v(Value::from(180_000_000i64))),
                // Capa opcional para testes: HALO_MOCK_ART=file:///caminho/capa.png
                ("mpris:artUrl".into(), v(Value::from(std::env::var("HALO_MOCK_ART").unwrap_or_default()))),
            ])
        }
        #[zbus(property)]
        fn position(&self) -> i64 {
            self.start.elapsed().as_micros() as i64
        }
        fn play_pause(&self) {}
        fn next(&self) {}
        fn previous(&self) {}
    }

    pub fn main() -> zbus::Result<()> {
        let _conn = zbus::blocking::connection::Builder::session()?
            .name("org.mpris.MediaPlayer2.mock")?
            .serve_at("/org/mpris/MediaPlayer2", Player { start: Instant::now() })?
            .build()?;
        loop {
            std::thread::park();
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn main() -> zbus::Result<()> {
    mpris::main()
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
fn main() {
    eprintln!("Este exemplo só existe no Linux (MPRIS).");
}
