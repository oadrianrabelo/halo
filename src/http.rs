//! Cliente HTTP compartilhado (letras e atualizações).
//!
//! Usa o TLS do sistema (SChannel no Windows, Security no macOS, OpenSSL no Linux) e os
//! certificados do sistema. O provedor precisa ser escolhido explicitamente: o padrão da
//! biblioteca é Rustls, que não está incluído, e isso travava o app na primeira conexão HTTPS.

use std::time::Duration;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

pub fn agent(timeout: Duration) -> ureq::Agent {
    let tls = TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(timeout))
        .user_agent(concat!("Halo/", env!("CARGO_PKG_VERSION"), " (+https://github.com/oadrianrabelo/halo)"))
        .build()
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Uma conexão HTTPS (mesmo que falhe) não pode derrubar o app.
    #[test]
    fn https_does_not_panic() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for s in listener.incoming().flatten() {
                drop(s);
            }
        });
        let result = agent(Duration::from_secs(5)).get(format!("https://127.0.0.1:{port}/")).call();
        assert!(result.is_err());
    }
}
