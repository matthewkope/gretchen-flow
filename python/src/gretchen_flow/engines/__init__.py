"""Engine registry: look up a TranscriptionEngine by name."""

from __future__ import annotations

import re

from .base import TranscriptionEngine

# A model identifier is a short name ("large-v3-turbo"), a Hugging Face repo id
# ("org/name"), or a local path. Reject path traversal and control/odd
# characters so a tampered config.json can't smuggle an unexpected value into
# the model loaders (which will download or load whatever they're handed).
_MODEL_RE = re.compile(r"\A[A-Za-z0-9._\-/:@+ ]+\Z")


def _validate_model(model: str) -> str:
    if not model or len(model) > 256 or ".." in model or _MODEL_RE.match(model) is None:
        raise ValueError(f"invalid model identifier: {model!r}")
    return model


def create_engine(name: str, model: str) -> TranscriptionEngine:
    model = _validate_model(model)
    if name == "faster-whisper":
        from .faster_whisper_engine import FasterWhisperEngine

        return FasterWhisperEngine(model)
    if name == "mlx-whisper":
        from .mlx_whisper_engine import MlxWhisperEngine

        return MlxWhisperEngine(model)
    raise ValueError(f"Unknown engine {name!r}. Available: faster-whisper, mlx-whisper")
