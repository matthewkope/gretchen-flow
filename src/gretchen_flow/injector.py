"""Deliver transcribed text into whatever app currently has focus.

Two strategies:
- "type": emit keystrokes via pynput. Works everywhere, slower for long text.
- "clipboard": put text on the clipboard and press Cmd+V (macOS). Fast, but
  overwrites the user's clipboard.

Both require macOS Accessibility permission for the terminal/app running GF.
"""

from __future__ import annotations

import subprocess
import sys
import time

from pynput.keyboard import Controller, Key

_keyboard = Controller()


def inject(text: str, strategy: str = "type") -> None:
    if not text:
        return
    if strategy == "clipboard":
        _paste(text)
    else:
        _keyboard.type(text)


def _paste(text: str) -> None:
    if sys.platform == "darwin":
        subprocess.run(["pbcopy"], input=text.encode(), check=True)
        time.sleep(0.05)
        with _keyboard.pressed(Key.cmd):
            _keyboard.press("v")
            _keyboard.release("v")
    else:
        # Clipboard paste is macOS-only for now; fall back to typing.
        _keyboard.type(text)
