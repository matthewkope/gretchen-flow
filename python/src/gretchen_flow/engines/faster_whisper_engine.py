"""Local transcription via faster-whisper (CTranslate2). Cross-platform default."""

from __future__ import annotations

import numpy as np

from .base import TranscriptionEngine


class FasterWhisperEngine(TranscriptionEngine):
    name = "faster-whisper"

    def __init__(self, model: str = "large-v3-turbo"):
        self._model_name = model
        self._model = None

    def warm_up(self) -> None:
        if self._model is None:
            from faster_whisper import WhisperModel

            # int8 keeps memory modest with near-identical accuracy on CPU/Apple Silicon.
            self._model = WhisperModel(self._model_name, compute_type="int8")

    def transcribe(self, audio: np.ndarray, sample_rate: int, language: str | None) -> str:
        self.warm_up()
        segments, _info = self._model.transcribe(
            audio,
            language=language,
            beam_size=5,
            vad_filter=True,
        )
        return " ".join(seg.text.strip() for seg in segments).strip()
