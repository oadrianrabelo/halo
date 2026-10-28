# Pomodoro

Um pomodoro flutuante, minúsculo e nativo para Windows, Linux e macOS.

- **Leve de verdade:** um único executável de ~6 MB, sem navegador nem WebView por baixo.
  Com o timer parado, não gasta CPU. Rodando, redesenha uma vez por segundo.
- **Flutuante:** fica sempre por cima das outras janelas, sem borda. Arraste por qualquer área vazia.
- **Personalizável:** foco, pausa, pausa longa, ciclos até a pausa longa, início automático e som.
  As configurações e a posição da janela ficam salvas.

## Atalhos

| Tecla  | Ação                    |
|--------|-------------------------|
| Espaço | Iniciar / pausar        |
| R      | Reiniciar a fase atual  |
| S      | Pular para a próxima    |

## Download

Baixe o executável do seu sistema na página de **Releases**:

- Windows: `pomodoro-windows-x64.exe`
- Linux: `pomodoro-linux-x64` (rode `chmod +x pomodoro-linux-x64` antes)
- macOS: `pomodoro-macos-arm64` (M1/M2/M3…) ou `pomodoro-macos-intel`

> No macOS o binário não é assinado. Na primeira vez, clique com o botão direito → Abrir.

## Compilar

Precisa do [Rust](https://rustup.rs).

```bash
cargo run --release
```

No Linux, instale antes: `libxkbcommon-dev libgl1-mesa-dev libwayland-dev`.

Para gerar uma release, crie uma tag `v*` (ex.: `git tag v0.1.0 && git push --tags`) e o GitHub Actions
compila e publica os binários dos três sistemas.

## Por que não Tauri/Electron?

Tauri usa a WebView do sistema. No Windows isso é o WebView2 (Chromium), que sobe vários processos e
costuma ocupar 50–100 MB de RAM. Aqui a interface é desenhada direto com [egui](https://github.com/emilk/egui)
via OpenGL, num processo só. Os ícones são desenhados com formas simples, então nem fonte de emoji é embutida.

## Estrutura

```
src/
  main.rs   janela, interface e atalhos
  timer.rs  lógica do pomodoro (sem UI, com testes)
  icons.rs  ícones desenhados à mão
  sound.rs  aviso sonoro usando o som do próprio sistema
```

## Ideias para o futuro

- **Música tocando agora:** no Windows pela API de mídia do sistema (GSMTC), no Linux via MPRIS (D-Bus),
  no macOS via MediaRemote. Vira um módulo novo (`src/media.rs`) que só é consultado quando o painel está aberto.
- **Letras sincronizadas:** buscar no [LRCLIB](https://lrclib.net) (API aberta, sem chave) pelo artista/título.
- Estatísticas simples de focos por dia.
