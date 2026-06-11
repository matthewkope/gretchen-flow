"""Microphone capture using sounddevice (PortAudio)."""

from __future__ import annotations

import threading

import numpy as np
import sounddevice as sd


class Recorder:
    """Start/stop microphone recording; returns mono float32 PCM."""

    def __init__(self, sample_rate: int = 16_000):
        self.sample_rate = sample_rate
        self._chunks: list[np.ndarray] = []
        self._stream: sd.InputStream | None = None
        self._lock = threading.Lock()

    @property
    def recording(self) -> bool:
        return self._stream is not None

    def start(self) -> None:
        with self._lock:
            if self._stream is not None:
                return
            self._chunks = []
            self._stream = sd.InputStream(
                samplerate=self.sample_rate,
                channels=1,
                dtype="float32",
                callback=self._on_audio,
            )
            self._stream.start()

    def _on_audio(self, indata, frames, time_info, status) -> None:
        self._chunks.append(indata[:, 0].copy())

    def stop(self) -> np.ndarray:
        """Stop recording and return everything captured since start()."""
        with self._lock:
            if self._stream is None:
                return np.zeros(0, dtype=np.float32)
            self._stream.stop()
            self._stream.close()
            self._stream = None
            if not self._chunks:
                return np.zeros(0, dtype=np.float32)
            audio = np.concatenate(self._chunks)
            self._chunks = []
            return audio
