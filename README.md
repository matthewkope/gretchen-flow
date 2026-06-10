# Gretchen Flow (GF)

**Press a hotkey, speak, and your words appear wherever your cursor is.**

Gretchen Flow is an open-source voice dictation app in the spirit of Wispr Flow:
a global hotkey starts recording, your speech is transcribed locally with
[Whisper](https://github.com/openai/whisper)-family models, and the text is typed
straight into whatever app has focus — your editor, browser, chat, anywhere.

- 🎙️ **Push-to-talk or toggle** — hold a key to talk, or tap to start/stop
- 🔒 **Local-first** — audio never leaves your machine by default
- 🎯 **Accuracy-focused** — defaults to `large-v3-turbo`; swap models freely
- 🔌 **Pluggable engines** — faster-whisper (cross-platform), mlx-whisper (Apple Silicon), cloud engines planned
- 🍎 macOS first; Linux/Windows support on the roadmap

## Quick start

Requires Python 3.10–3.13 and [uv](https://docs.astral.sh/uv/).

```bash
git clone https://github.com/matthewkope/gretchen-flow.git
cd gretchen-flow
uv sync                # or: uv sync --extra mlx   (Apple Silicon, faster)
uv run gf
```

Then press **Ctrl+Option+Space**, speak, press it again, and watch the text type
itself into the focused app. The first run downloads the model (~1.6 GB for
large-v3-turbo); use `--model small` for a quick lightweight test.

```bash
uv run gf --model small                 # smaller/faster model
uv run gf --engine mlx-whisper          # Apple Silicon native (needs --extra mlx)
uv run gf --mode hold                   # push-to-talk: record while held
uv run gf --hotkey "<cmd>+<shift>+d"    # custom hotkey
uv run gf --write-config               # persist current flags as defaults
```

Settings live in `~/.config/gretchen-flow/config.json`.

### macOS permissions

GF needs two permissions for the terminal app you run it from
(System Settings → Privacy & Security):

1. **Microphone** — to record your voice
2. **Accessibility** (and **Input Monitoring**) — to watch the global hotkey and
   type the transcribed text

macOS will prompt you on first run; after granting, restart GF.

## How it works

```
hotkey (pynput) ──> recorder (sounddevice, 16 kHz mono)
                          │ on stop
                          ▼
              transcription engine (pluggable)
        faster-whisper / mlx-whisper / cloud (planned)
                          │ text
                          ▼
            injector — keystrokes or clipboard paste
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the design options that were
considered and why this architecture was chosen.

## Roadmap

- [ ] Menu bar / tray icon with recording indicator
- [ ] Streaming transcription (text appears while you speak)
- [ ] Optional cloud engines (Deepgram, OpenAI, Groq) for max accuracy/speed
- [ ] Custom vocabulary and auto-formatting (punctuation, casing, lists)
- [ ] Linux and Windows support
- [ ] Packaged app (no Python required) — likely a Tauri shell, see ARCHITECTURE.md

## Contributing

PRs welcome! Dev setup:

```bash
uv sync
uv run pytest          # tests
uv run ruff check .    # lint
```

## License

[MIT](LICENSE)
