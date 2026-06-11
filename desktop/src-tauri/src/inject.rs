//! Type transcribed text into the focused app via synthetic keystrokes.
//! Requires macOS Accessibility permission.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

pub fn type_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("keyboard init failed (Accessibility permission?): {e}"))?;
    // Newlines (e.g. formatted lists) are sent as real Return keypresses;
    // many apps ignore a bare \n inside a synthetic text event.
    let mut parts = text.split('\n').peekable();
    while let Some(part) = parts.next() {
        if !part.is_empty() {
            enigo
                .text(part)
                .map_err(|e| format!("typing failed: {e}"))?;
        }
        if parts.peek().is_some() {
            enigo
                .key(Key::Return, Direction::Click)
                .map_err(|e| format!("typing failed: {e}"))?;
        }
    }
    Ok(())
}
