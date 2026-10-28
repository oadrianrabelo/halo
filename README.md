# Halo

Um pomodoro flutuante, bonito e minúsculo para Windows, Linux e macOS, com a música que está tocando e a letra sincronizada.

![Estilo Clássico](docs/classico.png)
![Estilo Fluido](docs/fluido.png)

- **Leve de verdade:** um único executável de ~7 MB, sem navegador nem WebView por baixo.
  Com o timer parado, não gasta CPU. Rodando, redesenha uma vez por segundo.
- **Flutuante:** fica por cima das outras janelas, sem borda. Arraste por qualquer lugar.
- **Dois estilos visuais**
  - **Clássico:** superfícies sólidas e discretas, botões vermelho/amarelo/verde.
  - **Fluido:** gradiente vivo que muda de cor com a fase, vidro, anel com brilho, números que rolam
    e transições suaves. Inspirado em apps como Arc, Raycast e Dia.
- **Muito personalizável:** 16 temas prontos, todas as cores editáveis, três layouts (Anel, Pílula, Mínimo),
  tamanho, cantos, opacidade, fonte (do sistema, embutida ou qualquer .ttf), peso dos números,
  efeito vidro nativo (Windows/macOS), som personalizado e o que aparece ou não na tela.
- **Tocando agora + letra sincronizada:** mostra a música do sistema (Spotify, navegador, etc.) com a linha
  atual da letra, buscada no [LRCLIB](https://lrclib.net). Controles de anterior, play/pausa e próxima ao passar o mouse.
- **Histórico:** focos de hoje, sequência de dias, meta diária, gráfico da semana e grade das últimas 16 semanas.
- Tudo fica salvo: configurações, tema, histórico e posição da janela.

## Como usar

| Ação                 | Como                                     |
|----------------------|------------------------------------------|
| Iniciar / pausar     | Clique no timer ou `Espaço`              |
| Reiniciar fase       | `R`                                      |
| Pular fase           | `S`                                      |
| Mover                | Arraste                                  |
| Preferências         | Botão direito, `Ctrl/⌘ + ,` ou o botão que aparece ao passar o mouse |
| Sair                 | `Ctrl/⌘ + Q`                             |

Os controles somem enquanto o timer roda e voltam quando o mouse passa por cima (dá para desligar isso).

![Preferências](docs/preferencias-fluido.png)

## Download

Baixe o executável do seu sistema na página de **Releases**:

- Windows: `halo-windows-x64.exe`
- Linux: `halo-linux-x64` (rode `chmod +x halo-linux-x64` antes)
- macOS: `halo-macos-arm64` (Apple Silicon) ou `halo-macos-intel`

> No macOS o binário não é assinado. Na primeira vez, clique com o botão direito → Abrir.

## Compilar

Precisa do [Rust](https://rustup.rs).

```bash
cargo run --release
```

No Linux, instale antes: `libxkbcommon-dev libgl1-mesa-dev libwayland-dev libssl-dev pkg-config`.

Para gerar uma release, crie uma tag `v*` (ex.: `git tag v0.3.0 && git push --tags`) e o GitHub Actions
compila e publica os binários dos três sistemas.

## Por que não Tauri/Electron?

Tauri usa a WebView do sistema. No Windows isso é o WebView2 (Chromium), que sobe vários processos e
costuma ocupar 50–100 MB de RAM. Aqui a interface é desenhada direto com [egui](https://github.com/emilk/egui)
via OpenGL, num processo só. Ícones, gradientes e brilhos são desenhados com formas e malhas simples,
então não há imagens nem fontes de ícone embutidas. A fonte é lida do próprio sistema.

As animações do estilo Fluido (números rolando, troca de cor) rodam por uma fração de segundo
a cada mudança e podem ser desligadas em Preferências → Aparência.

## Música e letras

![Tocando agora com letra](docs/musica.png)

| Sistema | De onde vem a música | Observação |
|---------|----------------------|------------|
| Windows | Controles de mídia do Windows (os mesmos do painel de volume) | Funciona com Spotify, navegadores, Música… |
| Linux   | MPRIS (D-Bus) | Spotify, navegadores, VLC, Rhythmbox, Elisa… |
| macOS   | AppleScript | Spotify e Música. Na primeira vez o macOS pede permissão. |

A leitura roda numa thread separada, uma vez por segundo. A tela só é redesenhada quando a linha da letra
muda. Dá para desligar tudo em Preferências → Música.

Para testar no Linux sem um player de verdade, há um player MPRIS falso:

```bash
dbus-run-session -- sh -c 'cargo run --example mock_player & sleep 1; cargo run'
```

`HALO_LYRICS_API` aponta as letras para outro servidor compatível com o LRCLIB (útil em testes).

## Estrutura

```
src/
  main.rs     cria a janela
  app.rs      janela principal: layouts, interação, animações, painel de música
  prefs.rs    janela de Preferências
  timer.rs    lógica do pomodoro (sem UI, com testes)
  stats.rs    histórico de focos por dia
  media.rs    música tocando no sistema (Windows, Linux, macOS)
  lyrics.rs   letras sincronizadas (LRCLIB) e leitura de .lrc
  theme.rs    temas, estilos e tudo que é personalizável
  widgets.rs  ícones, anel, interruptor, controle segmentado…
  fluid.rs    gradiente, vidro e brilho do estilo Fluido
  fonts.rs    carrega a fonte do sistema ou um arquivo
  glass.rs    desfoque nativo (Windows/macOS)
  sound.rs    aviso sonoro usando o som do próprio sistema
```
