"""Transcription engine interface.

Engines are pluggable so GF can swap between local models (faster-whisper,
mlx-whisper) and, later, cloud APIs — without touching the rest of the app.
"""

from __future__ import annotations

from abc import ABC, abstractmethod

import numpy as np


class TranscriptionEngine(ABC):
    """Turns a mono float32 PCM buffer into text."""

    name: str

    @abstractmethod
    def transcribe(self, audio: np.ndarray, sample_rate: int, language: str | None) -> str:
        """Transcribe `audio` (1-D float32, range [-1, 1]) and return the text."""

    def warm_up(self) -> None:  # noqa: B027 - optional hook, not abstract
        """Optional: load the model ahead of the first transcription."""
