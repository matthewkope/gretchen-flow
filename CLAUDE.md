# Gretchen Flow — working notes for Claude

Open-source macOS push-to-talk dictation (Wispr Flow-style): hold a hotkey,
speak, and the transcribed text is typed at the cursor. Fully **local** — audio
never leaves the machine, **no API keys, no cloud**. Keep it that way.

## Layout

| Path | What |
|---|---|
| `desktop/src-tauri/` | The app — Tauri 2 + Rust. This is where almost all work happens. |
| `desktop/ui/` | Webview windows: `menu.html`/`menu.js` (main NES-style window), `setup.html`/`setup.js` (first-run guide), `hotkey.html`/`hotkey.js` (shortcut recorder). HTML loads external JS (CSP in `tauri.conf.json` forbids inline script). |
| `desktop/scripts/gen_icons.py` | Generates tray + app icons from `icons/gretchen-source.png`. |
| `python/` | The original Python prototype. Still works; not the product. |
| `Casks/gretchen-flow.rb` | Homebrew cask. `.github/workflows/release.yml` builds the DMG on a `v*` tag. |

## Build / run

```bash
cd desktop/src-tauri
cargo run                 # dev: raw unsigned binary (fast iteration)
cargo build               # compile-check
cargo clippy              # keep clean (0 warnings)
cargo test                # transcribe.rs has the test suite
cargo tauri build --debug # produce a signed .app bundle + DMG

uv run desktop/scripts/gen_icons.py   # regenerate icons (from repo root)
cd python && uv run pytest            # prototype tests
```

To test the **bundled** app (Dock icon, status item behavior), build the bundle
and run `/Applications/Gretchen Flow.app` — the raw `cargo run` binary has the
generic exec Dock icon and flaky status-item behavior.

## Architecture (`desktop/src-tauri/src/`)

- `main.rs` (~1k lines) — everything UI/state: `AppState`, tray `set_tray_state`,
  the `build_menu`/`refresh_menu` tray menu (mutated in place — never rebuilt, or
  the dropdown glitches on hover), the standalone window openers, all `#[tauri::command]`
  IPC, shortcut registration, and the Fn listener.
- `transcribe.rs` — whisper-rs engine: pause-punctuation, filler removal, list
  formatting. Has the tests.
- `audio.rs` (cpal), `inject.rs` (enigo keystrokes), `model.rs` (download/resolve
  ggml models), `config.rs`, `history.rs`, `lists.rs`.

Tray icons are **embedded** via `include_bytes!` (`ICON_IDLE`, `ICON_RECORDING`),
so **any icon change requires a recompile**.

### Hotkeys
Up to 3 shortcuts (`AppState.shortcuts`, config `shortcuts: ["Fn"]`, default Fn).
Normal accelerators go through `tauri-plugin-global-shortcut`. **"Fn"/Globe** is
special — macOS can't register it, so `spawn_fn_listener` runs a low-level
`CGEventTap` watching the `SecondaryFn` flag (needs Input Monitoring). Don't add
keyboard-layout calls in that tap — it must stay off the main thread.

## macOS gotchas (these caused real pain — read before touching)

- **Permissions reset on every rebuild.** The app is ad-hoc signed, so each
  rebuild is a new code identity → macOS wipes **Microphone**, **Accessibility**,
  and **Input Monitoring** grants. A trusted/untrusted self-signed cert did *not*
  reliably fix this on the dev machine. Practical rule: **minimize rebuilds**, and
  after one, re-grant (hold the hotkey → Allow popup for mic; toggle the other two).
  When the mic is denied, recordings are silent and Whisper hallucinates the
  caption phrase **"Sous-titrage Société Radio-Canada"** — that string == no mic.
- **`signingIdentity: "Gretchen Flow Dev"` in `tauri.conf.json` is a LOCAL dev
  cert** (in the user's login keychain only). It MUST be set back to `"-"` (or a
  real Developer ID) before a public release / for CI, or `cargo tauri build`
  fails elsewhere.
- **Menu-bar notch.** A new status item lands in the leftmost slot, which on a
  notched Mac is *behind the notch* (invisible). macOS controls placement — no
  code fix. The durable fix is a one-time **⌘-drag** (persists by bundle id).
- **The orange mic** while recording is macOS's own privacy indicator — apps
  cannot move, hide, or replace it.
- **Window dragging** needs `capabilities/default.json`
  (`core:window:allow-start-dragging`); without it, `data-tauri-drag-region` is
  silently denied.
- Native AppKit (NSOpenPanel, NSPasteboard, setApplicationIconImage) is done via
  objc2; run it on the main thread (`run_on_main_thread`).

## Conventions

- Local-only, no network except model downloads from Hugging Face. No telemetry.
- Config: `~/.config/gretchen-flow/config.json`. Models: `~/.cache/gretchen-flow/models/`.
  History: `~/Library/Application Support/gretchen-flow/history.jsonl`.
- Commit only when asked; branch off the default branch first. End commit messages
  with the Co-Authored-By trailer.
