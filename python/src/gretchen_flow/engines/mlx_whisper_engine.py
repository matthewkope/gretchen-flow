"""Local transcription via mlx-whisper — fastest option on Apple Silicon.

Install with: uv sync --extra mlx
"""

from __future__ import annotations

import numpy as np

from .base import TranscriptionEngine

# Whisper model names -> MLX community conversions on the Hugging Face Hub.
_MLX_REPOS = {
    "large-v3-turbo": "mlx-community/whisper-large-v3-turbo",
    "large-v3": "mlx-community/whisper-large-v3-mlx",
    "medium": "mlx-community/whisper-medium-mlx",
    "small": "mlx-community/whisper-small-mlx",
    "base": "mlx-community/whisper-base-mlx",
    "tiny": "mlx-community/whisper-tiny-mlx",
}


class MlxWhisperEngine(TranscriptionEngine):
    name = "mlx-whisper"

    def __init__(self, model: str = "large-v3-turbo"):
        self._repo = _MLX_REPOS.get(model, model)

    def transcribe(self, audio: np.ndarray, sample_rate: int, language: str | None) -> str:
        import mlx_whisper

        result = mlx_whisper.transcribe(
            audio,
            path_or_hf_repo=self._repo,
            language=language,
        )
        return result["text"].strip()
