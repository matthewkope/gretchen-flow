# Gretchen Flow (GF)

**Press a hotkey, speak, and your words appear wherever your cursor is.**

Gretchen Flow is an open-source voice dictation app in the spirit of Wispr Flow:
a global hotkey starts recording, your speech is transcribed locally with
Whisper, and the text is typed straight into whatever app has focus — your
editor, browser, chat, anywhere.

- ¿ **Lives in the menu bar** — the icon shows state at a glance: ¿ idle, ● recording, … transcribing
- 🎙️ **Push-to-talk or toggle** — hold a key to talk, or tap to start/stop
- 🔒 **Local-first** — audio never leaves your machine
- 🎯 **Accuracy-focused** — Whisper with Metal acceleration on Apple Silicon
- 🦀 **Native** — Tauri 2 + Rust; small binary, low latency

## Repository layout

| Directory | What it is |
|---|---|
| [`desktop/`](desktop/) | **The app** — Tauri 2 + Rust menu-bar application |
| [`python/`](python/) | The original Python prototype (still works; great for experimenting with models) |
| [`docs/`](docs/) | Architecture and design decisions |

## Quick start (desktop app)

Requires [Rust](https://rustup.rs) and cmake (`brew install cmake`).

```bash
git clone https://github.com/matthewkope/gretchen-flow.git
cd gretchen-flow/desktop/src-tauri
cargo run
```

A **¿** appears in your menu bar. The first run downloads the Whisper model
(~470 MB for `small`), shown as **¿↓**. When the arrow disappears:

1. Click into any text field
2. **Hold Ctrl+Option+Space** — the ¿ lights up bold red while it listens
3. Speak, then **let go** — the ¿ turns amber while transcribing, then your
   words are typed where your cursor is

| Menu bar | Meaning |
|---|---|
| ¿ | idle, ready |
| ¿↓ | downloading the model (first run) |
| **¿** (bold red, glowing) | recording — release the keys to finish |
| **¿** (amber) | transcribing |
| ¿✕ | model failed to load (check the log) |

### Configuration

`~/.config/gretchen-flow/config.json`:

```json
{
  "model": "small",
  "language": "en",
  "shortcut": "Ctrl+Alt+Space",
  "hotkey_mode": "hold"
}
```

- `model`: any ggml model from [whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp)
  — `base`, `small`, `medium`, `large-v3-turbo` (best accuracy, ~1.6 GB)
- `hotkey_mode`: `"hold"` (push-to-talk — records while held, default) or `"toggle"` (tap to start/stop)
- `shortcut`: any [Tauri accelerator](https://v2.tauri.app/learn/global-shortcut/), e.g. `"Cmd+Shift+D"`

### macOS permissions

System Settings → Privacy & Security — the app (or your terminal, when using
`cargo run`) needs:

1. **Microphone** — to hear you
2. **Accessibility** — to type the transcribed text into other apps

## Roadmap

- [ ] Settings window (model picker, shortcut recorder)
- [ ] Streaming transcription (text appears while you speak)
- [ ] Custom vocabulary and auto-formatting
- [ ] Signed .dmg releases
- [ ] Linux and Windows support

## Contributing

PRs welcome! See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the design.

```bash
cd desktop/src-tauri && cargo build   # the app
cd python && uv sync && uv run pytest # the prototype
```

## License

[MIT](LICENSE)
