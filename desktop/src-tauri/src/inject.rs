//! Type transcribed text into the focused app via synthetic keystrokes.
//! Requires macOS Accessibility permission.

use enigo::{Enigo, Keyboard, Settings};

pub fn type_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("keyboard init failed (Accessibility permission?): {e}"))?;
    enigo.text(text).map_err(|e| format!("typing failed: {e}"))
}
