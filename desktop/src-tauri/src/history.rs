//! Dictation history: each transcription is appended to a JSONL file so the
//! tray menu can offer recent entries for re-typing (and nothing is ever lost
//! to a misplaced cursor).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn history_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gretchen-flow/history.jsonl")
}

pub fn append(text: &str) {
    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let entry = serde_json::json!({ "ts": ts, "text": text });
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(file, "{entry}");
    }
}

/// The most recent `n` transcriptions, newest first.
pub fn recent(n: usize) -> Vec<String> {
    let Ok(content) = fs::read_to_string(history_path()) else {
        return Vec::new();
    };
    let mut entries: Vec<String> = content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|v| v["text"].as_str().map(String::from))
        .collect();
    entries.reverse();
    entries.truncate(n);
    entries
}
