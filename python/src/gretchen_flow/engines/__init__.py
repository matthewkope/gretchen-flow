"""Engine registry: look up a TranscriptionEngine by name."""

from __future__ import annotations

from .base import TranscriptionEngine


def create_engine(name: str, model: str) -> TranscriptionEngine:
    if name == "faster-whisper":
        from .faster_whisper_engine import FasterWhisperEngine

        return FasterWhisperEngine(model)
    if name == "mlx-whisper":
        from .mlx_whisper_engine import MlxWhisperEngine

        return MlxWhisperEngine(model)
    raise ValueError(f"Unknown engine {name!r}. Available: faster-whisper, mlx-whisper")
