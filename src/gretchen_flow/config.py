"""User configuration, loaded from ~/.config/gretchen-flow/config.json."""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from pathlib import Path

CONFIG_DIR = Path.home() / ".config" / "gretchen-flow"
CONFIG_PATH = CONFIG_DIR / "config.json"


@dataclass
class Config:
    # Engine: "faster-whisper" (default, cross-platform) or "mlx-whisper" (Apple Silicon).
    engine: str = "faster-whisper"
    # Model size/name. "large-v3-turbo" is the best accuracy/speed tradeoff;
    # use "small" or "base" on machines with limited RAM.
    model: str = "large-v3-turbo"
    # BCP-47 language code, or None to auto-detect.
    language: str | None = "en"
    # Global hotkey (pynput syntax). Hold to talk, release to transcribe.
    hotkey: str = "<ctrl>+<alt>+<space>"
    # "hold" (push-to-talk) or "toggle" (press to start, press again to stop).
    hotkey_mode: str = "toggle"
    # How the text is delivered: "type" (keystrokes) or "clipboard" (paste via Cmd+V).
    injection: str = "type"
    sample_rate: int = 16_000
    extra: dict = field(default_factory=dict)

    @classmethod
    def load(cls, path: Path = CONFIG_PATH) -> Config:
        if path.exists():
            data = json.loads(path.read_text())
            known = {f for f in cls.__dataclass_fields__}
            kwargs = {k: v for k, v in data.items() if k in known}
            kwargs["extra"] = {k: v for k, v in data.items() if k not in known}
            return cls(**kwargs)
        return cls()

    def save(self, path: Path = CONFIG_PATH) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        data = asdict(self)
        data.update(data.pop("extra"))
        path.write_text(json.dumps(data, indent=2) + "\n")
