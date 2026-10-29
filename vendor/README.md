# vendor/

## egui-winit 0.37.0

Cópia do [egui-winit](https://github.com/emilk/egui) 0.37.0 (MIT OR Apache-2.0) com **uma** mudança,
marcada com `HALO:` em `src/lib.rs`: no Windows, `ViewportBuilder::with_has_shadow(false)` passa a
desligar a "sombra de janela sem moldura" do winit. O original sempre liga essa sombra, e ela faz o
Windows reservar 1 px no topo e estender a moldura — o que aparece como borda em volta de uma janela
transparente.

Usada por `[patch.crates-io]` no `Cargo.toml`. Ao atualizar o egui, copie a versão nova e reaplique
a mudança (procure por `want_undecorated_shadow`).
