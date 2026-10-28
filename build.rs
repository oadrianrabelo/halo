//! No Windows, embute o ícone e as informações de versão no .exe.
fn main() {
    println!("cargo:rerun-if-changed=assets/halo.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/halo.ico")
            .set("ProductName", "Halo")
            .set("FileDescription", "Halo")
            .set("LegalCopyright", "Halo");
        if let Err(e) = res.compile() {
            println!("cargo:warning=não consegui embutir o ícone do Windows: {e}");
        }
    }
}
