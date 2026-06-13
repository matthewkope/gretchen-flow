# Gretchen Flow (GF)

**Press a hotkey, speak, and your words appear wherever your cursor is.**

Gretchen Flow is an open-source voice dictation app in the spirit of Wispr Flow:
a global hotkey starts recording, your speech is transcribed locally with
Whisper, and the text is typed straight into whatever app has focus — your
editor, browser, chat, anywhere.

- 👧 **Gretchen lives in the menu bar** — quiet when idle, glowing red while recording, amber while transcribing
- 🎙️ **Push-to-talk or toggle** — hold a key to talk, or tap to start/stop
- ✍️ **Speech-aware grammar** — pausing while you speak inserts a period; filler
  words ("um", "uh") are removed automatically
- 📋 **Spoken lists** — "one, buy milk… two, walk the dog" becomes a real
  numbered list, typed line by line
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

## Install with Homebrew

Apple Silicon Mac, macOS 12+:

```bash
brew tap matthewkope/gretchen-flow
brew trust matthewkope/gretchen-flow   # Homebrew requires trusting any third-party cask tap
brew install --cask gretchen-flow
```

> The cask installs from the latest [GitHub Release](https://github.com/matthewkope/gretchen-flow/releases).
> Builds are not yet notarized — if macOS blocks the app on first launch, run
> `xattr -dr com.apple.quarantine "/Applications/Gretchen Flow.app"`.

## Build from source

Requires [Rust](https://rustup.rs) and cmake (`brew install cmake`).

```bash
git clone https://github.com/matthewkope/gretchen-flow.git
cd gretchen-flow/desktop/src-tauri
cargo run
```

## First run — download a model

**Gretchen Flow ships with no speech model**, so you pick the one you want.
On first launch a **setup window** opens (the menu bar icon shows a small **!**
until a model is ready). To get going:

1. Click **Download Recommended Model (547 MB)** in the setup window — or open
   the menu bar icon's **Model** submenu and choose one. The recommended
   **Large v3 Turbo (quantized)** is the best balance of accuracy and size.
   The Gretchen icon shows a **↓** while it downloads.
2. **Grant permissions** when macOS prompts: **Microphone** (to hear you) and
   **Accessibility** (to type the text). Both are under
   System Settings ▸ Privacy & Security.
3. **Dictate:** click into any text field, **hold Ctrl+Option+Space** —
   Gretchen lights up while she listens — then **let go**; your words type out
   where the cursor is.

You can reopen the guide anytime from the menu's **Getting Started…** item.

| Menu bar (Gretchen icon) | Meaning |
|---|---|
| monochrome | idle, ready |
| with ! | no model yet — open the setup guide / Model menu |
| with ↓ | downloading a model |
| **red/amber, glowing** | recording — release the keys to finish |
| **amber** | transcribing |
| with ✕ | model failed to load (check the log) |

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
  "auto_lists": true,
  "vocabulary": ["Gretchen Flow"]
}
```

- `model`: empty on a fresh install (pick one from the **Model** menu, which
  downloads it). Accepts any ggml model name from
  [whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp) —
  `large-v3-turbo-q5_0` (recommended, ~547 MB), `large-v3-turbo` (~1.6 GB),
  `small` (~466 MB), `base` (~142 MB) — or an absolute path to a local
  `.bin`/`.gguf` model file (also selectable via **Model ▸ Model from File…**)
- `hotkey_mode`: `"hold"` (push-to-talk — records while held, default) or `"toggle"` (tap to start/stop)
- `shortcut`: any [Tauri accelerator](https://v2.tauri.app/learn/global-shortcut/), e.g. `"Cmd+Shift+D"` —
  or just pick one from the tray menu's **Hotkey** submenu (takes effect
  immediately and is saved here)
- `pause_punctuation_ms`: pausing this long while speaking inserts a period and
  capitalizes the next sentence (smaller models often skip punctuation; this
  recovers it from your speech rhythm). Set `0` to disable.
- `remove_fillers`: strips "um", "uh", "hmm", etc. from the output
- `auto_lists`: formats spoken lists ("one, … two, …" / "first, … second, …" /
  "number one, …") as numbered lines; conservative, so narrative sentences like
  "One day I went out" are left alone
- `vocabulary`: words and phrases (names, jargon, brands) that recognition is
  biased toward, so they come out spelled the way you wrote them

Most settings can also be changed from the tray menu (model, hotkey, icon
theme). After hand-editing `config.json`, choose **Reload Config** from the
tray menu to apply every change live — no restart needed.

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
