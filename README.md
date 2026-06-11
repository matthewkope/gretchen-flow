# Gretchen Flow (GF)

**Press a hotkey, speak, and your words appear wherever your cursor is.**

Gretchen Flow is an open-source voice dictation app in the spirit of Wispr Flow:
a global hotkey starts recording, your speech is transcribed locally with
Whisper, and the text is typed straight into whatever app has focus — your
editor, browser, chat, anywhere.

- ¿ **Lives in the menu bar** — quiet ¿ when idle, bold glowing red while recording, amber while transcribing
- 🎙️ **Push-to-talk or toggle** — hold a key to talk, or tap to start/stop
- ✍️ **Speech-aware grammar** — pausing while you speak inserts a period; filler
  words ("um", "uh") are removed automatically
- 📖 **Personal dictionary** — add names and jargon so they're recognized and
  spelled right
- 🕘 **History** — recent dictations live in the tray menu; click one to type it again
- 🔒 **Local-first** — audio never leaves your machine
- 🎯 **Accurate and lightweight** — quantized Whisper large-v3-turbo (~574 MB)
  with Metal acceleration on Apple Silicon
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
(~574 MB for the default `large-v3-turbo-q5_0`), shown as **¿↓**. When the
arrow disappears:

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
  "model": "large-v3-turbo-q5_0",
  "language": "en",
  "shortcut": "Ctrl+Alt+Space",
  "hotkey_mode": "hold",
  "pause_punctuation_ms": 700,
  "remove_fillers": true,
  "vocabulary": ["Gretchen Flow"]
}
```

- `model`: any ggml model from [whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp)
  — `large-v3-turbo-q5_0` (default: near-flagship accuracy, ~574 MB),
  `large-v3-turbo` (~1.6 GB), `small` (~466 MB, fast/lighter), `base` (~142 MB)
- `hotkey_mode`: `"hold"` (push-to-talk — records while held, default) or `"toggle"` (tap to start/stop)
- `shortcut`: any [Tauri accelerator](https://v2.tauri.app/learn/global-shortcut/), e.g. `"Cmd+Shift+D"`
- `pause_punctuation_ms`: pausing this long while speaking inserts a period and
  capitalizes the next sentence (smaller models often skip punctuation; this
  recovers it from your speech rhythm). Set `0` to disable.
- `remove_fillers`: strips "um", "uh", "hmm", etc. from the output
- `vocabulary`: words and phrases (names, jargon, brands) that recognition is
  biased toward, so they come out spelled the way you wrote them

Dictation history is saved to `~/Library/Application Support/gretchen-flow/history.jsonl`;
the five most recent entries appear in the tray menu — click one to type it again.

### macOS permissions

System Settings → Privacy & Security — the app (or your terminal, when using
`cargo run`) needs:

1. **Microphone** — to hear you
2. **Accessibility** — to type the transcribed text into other apps

## Roadmap

- [ ] Settings window (model picker, shortcut recorder, dictionary editor)
- [ ] Streaming transcription (text appears while you speak)
- [ ] Self-correction cleanup ("Tuesday — no wait, Wednesday" → "Wednesday")
- [ ] Per-app tone/formatting awareness
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
