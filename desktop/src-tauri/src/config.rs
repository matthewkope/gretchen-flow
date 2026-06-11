//! User configuration, shared with the Python prototype at
//! `~/.config/gretchen-flow/config.json`. The desktop app uses the `shortcut`
//! key (Tauri accelerator syntax); the Python app's `hotkey` key is left alone.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// ggml model name from ggerganov/whisper.cpp, e.g. "small", "large-v3-turbo".
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
    /// Keep unknown keys (e.g. the Python app's settings) intact on save.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "small".into(),
            language: "en".into(),
            shortcut: "Ctrl+Alt+Space".into(),
            hotkey_mode: "hold".into(),
            pause_punctuation_ms: 700,
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
}
