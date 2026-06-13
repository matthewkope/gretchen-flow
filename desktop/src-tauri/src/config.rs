//! User configuration, shared with the Python prototype at
//! `~/.config/gretchen-flow/config.json`. The desktop app uses the `shortcut`
//! key (Tauri accelerator syntax); the Python app's `hotkey` key is left alone.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// ggml model name from ggerganov/whisper.cpp (e.g. "large-v3-turbo-q5_0")
    /// or an absolute path to a model file. Empty on a fresh install — the app
    /// ships with no model and guides the user to download one.
    pub model: String,
    /// Language code, or "auto" to detect.
    pub language: String,
    /// Global shortcut in Tauri accelerator syntax.
    pub shortcut: String,
    /// "toggle" (press to start/stop) or "hold" (push-to-talk).
    pub hotkey_mode: String,
    /// Insert a period when the speaker pauses at least this long (ms).
    /// 0 disables pause punctuation.
    pub pause_punctuation_ms: u64,
    /// Strip filler words ("um", "uh", ...) from transcriptions.
    pub remove_fillers: bool,
    /// Format spoken lists ("one, ... two, ..." / "first, ... second, ...")
    /// as numbered lines.
    pub auto_lists: bool,
    /// Personal dictionary: names and jargon to bias recognition toward,
    /// e.g. ["Gretchen Flow", "Tauri", "Kope"].
    pub vocabulary: Vec<String>,
    /// Idle tray badge: "dark" (white art on black) or "light" (black on
    /// white). Clicking the tray icon toggles and saves this.
    pub icon_theme: String,
    /// Keep unknown keys (e.g. the Python app's settings) intact on save.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Quantized large-v3-turbo: near-flagship accuracy at ~574 MB.
            model: String::new(),
            language: "en".into(),
            shortcut: "Ctrl+Alt+Space".into(),
            hotkey_mode: "hold".into(),
            pause_punctuation_ms: 700,
            remove_fillers: true,
            auto_lists: true,
            vocabulary: vec!["Gretchen Flow".into()],
            icon_theme: "dark".into(),
            extra: serde_json::Map::new(),
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/gretchen-flow/config.json")
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json + "\n");
        }
    }
}
