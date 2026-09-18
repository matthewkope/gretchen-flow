//! User configuration, shared with the Python prototype at
//! `~/.config/gretchen-flow/config.json`. The desktop app uses the `shortcuts`
//! key (a list of Tauri accelerators, or "Fn"); the Python app's `hotkey` key
//! is left alone.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_MODEL: &str = "parakeet-tdt-0.6b-v2";

pub fn storage_name() -> &'static str {
    if cfg!(feature = "test-build") {
        "gretchen-flow-test"
    } else {
        "gretchen-flow"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Parakeet v2 (default), a ggml Whisper model name, or an absolute ggml path.
    pub model: String,
    /// Language code, or "auto" to detect.
    pub language: String,
    /// Active push-to-talk shortcuts (up to 3). "Fn" means the Fn/Globe key;
    /// any other entry is a Tauri accelerator (e.g. "Cmd+Shift+Space").
    pub shortcuts: Vec<String>,
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
            model: DEFAULT_MODEL.into(),
            language: "en".into(),
            // Fn/Globe is the default push-to-talk key; users add up to 2 more.
            shortcuts: vec!["Fn".into()],
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
        .join(".config")
        .join(storage_name())
        .join("config.json")
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Self>(&s).ok())
            .map(|mut cfg| {
                if cfg.model.is_empty() {
                    cfg.model = DEFAULT_MODEL.into();
                }
                cfg
            })
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_config_defaults_to_parakeet() {
        assert_eq!(Config::default().model, DEFAULT_MODEL);
        assert_eq!(
            serde_json::from_str::<Config>("{}").unwrap().model,
            DEFAULT_MODEL
        );
    }
    #[test]
    fn explicit_whisper_choice_and_unknown_settings_survive() {
        let cfg: Config =
            serde_json::from_str(r#"{"model":"large-v3-turbo","custom":true}"#).unwrap();
        assert_eq!(cfg.model, "large-v3-turbo");
        assert_eq!(serde_json::to_value(cfg).unwrap()["custom"], true);
    }
}
