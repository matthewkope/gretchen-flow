//! Dictation history: each transcription is appended to a JSONL file so the
//! tray menu can offer recent entries for re-typing (and nothing is ever lost
//! to a misplaced cursor).
//!
//! Dictations are sensitive (they can contain passwords, messages, etc.) and
//! are stored in cleartext, so history is capped to the most recent
//! `HISTORY_CAP` entries rather than growing without bound, and can be cleared
//! from the menu.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Keep at most this many recent dictations on disk.
const HISTORY_CAP: usize = 200;

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
    trim_to_cap(&path);
}

/// Keep the history file from growing without bound: if it exceeds the cap,
/// rewrite it with only the most recent `HISTORY_CAP` lines (atomically, via a
/// temp file + rename, so a crash can't leave a half-written history).
fn trim_to_cap(path: &std::path::Path) {
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= HISTORY_CAP {
        return;
    }
    let kept = &lines[lines.len() - HISTORY_CAP..];
    let tmp = path.with_extension("jsonl.tmp");
    if fs::write(&tmp, kept.join("\n") + "\n").is_ok() {
        let _ = fs::rename(&tmp, path);
    }
}

/// Delete all stored dictation history.
pub fn clear() {
    let path = history_path();
    match fs::remove_file(&path) {
        Ok(()) => log::info!("dictation history cleared"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => log::error!("couldn't clear history: {e}"),
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
