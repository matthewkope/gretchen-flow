"""Global hotkey listener supporting toggle and hold (push-to-talk) modes."""

from __future__ import annotations

from collections.abc import Callable

from pynput import keyboard


class HotkeyListener:
    """Watch for a global key combo and fire start/stop callbacks.

    - mode="toggle": full combo press starts recording; the next press stops it.
    - mode="hold":   recording runs while the combo is held down.
    """

    def __init__(
        self,
        combo: str,
        mode: str,
        on_start: Callable[[], None],
        on_stop: Callable[[], None],
    ):
        self._combo = set(keyboard.HotKey.parse(combo))
        self._mode = mode
        self._on_start = on_start
        self._on_stop = on_stop
        self._pressed: set = set()
        self._active = False
        self._listener = keyboard.Listener(
            on_press=self._handle_press, on_release=self._handle_release
        )

    def start(self) -> None:
        self._listener.start()

    def join(self) -> None:
        self._listener.join()

    def stop(self) -> None:
        self._listener.stop()

    def _canonical(self, key):
        return self._listener.canonical(key)

    def _handle_press(self, key) -> None:
        k = self._canonical(key)
        if k not in self._combo or k in self._pressed:
            return
        self._pressed.add(k)
        if self._pressed != self._combo:
            return
        if self._mode == "hold":
            if not self._active:
                self._active = True
                self._on_start()
        else:
            self._active = not self._active
            (self._on_start if self._active else self._on_stop)()

    def _handle_release(self, key) -> None:
        k = self._canonical(key)
        self._pressed.discard(k)
        if self._mode == "hold" and self._active and k in self._combo:
            self._active = False
            self._on_stop()
