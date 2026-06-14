const { invoke } = window.__TAURI__.core;
const combo = document.getElementById("combo");
const error = document.getElementById("error");
const save = document.getElementById("save");
let accel = null;

const SYMBOLS = { Cmd: "⌘", Ctrl: "⌃", Alt: "⌥", Shift: "⇧" };
const KEYMAP = {
  Space: "Space", Enter: "Enter", Tab: "Tab", Backspace: "Backspace",
  Delete: "Delete", ArrowUp: "Up", ArrowDown: "Down",
  ArrowLeft: "Left", ArrowRight: "Right",
  Minus: "-", Equal: "=", Comma: ",", Period: ".", Slash: "/",
  Semicolon: ";", Quote: "'", BracketLeft: "[", BracketRight: "]",
  Backslash: "\\", Backquote: "`",
};

function keyName(e) {
  const c = e.code;
  if (c.startsWith("Key")) return c.slice(3);
  if (c.startsWith("Digit")) return c.slice(5);
  if (/^F\d+$/.test(c)) return c;
  return KEYMAP[c] || null;
}

window.addEventListener("keydown", (e) => {
  e.preventDefault();
  if (e.key === "Escape") { invoke("cancel_custom_hotkey"); return; }
  error.textContent = "";
  const mods = [];
  if (e.metaKey) mods.push("Cmd");
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  const key = keyName(e);
  if (!key) {
    // Modifier-only so far — show progress.
    combo.textContent = mods.length
      ? mods.map((m) => SYMBOLS[m]).join("") + " …" : "…";
    save.disabled = true; accel = null;
    return;
  }
  if (!mods.length && !/^F\d+$/.test(key)) {
    combo.textContent = "…";
    error.textContent = "Add a modifier (⌘ ⌃ ⌥ ⇧) or use an F-key";
    save.disabled = true; accel = null;
    return;
  }
  accel = [...mods, key].join("+");
  combo.textContent = mods.map((m) => SYMBOLS[m]).join("") + " " + key;
  save.disabled = false;
});

save.onclick = () => {
  if (!accel) return;
  invoke("apply_custom_hotkey", { accel }).catch((err) => {
    error.textContent = String(err);
  });
};
document.getElementById("fn").onclick = () =>
  invoke("apply_custom_hotkey", { accel: "Fn" }).catch((err) => {
    error.textContent = String(err);
  });
document.getElementById("cancel").onclick = () =>
  invoke("cancel_custom_hotkey");
document.getElementById("remove").onclick = () =>
  invoke("remove_pending_hotkey");

// If we were opened to change an existing shortcut, adjust the wording
// and reveal the Remove button.
invoke("hotkey_replace_target").then((target) => {
  if (!target) return;
  const shown = target === "Fn" ? "Fn 🌐" : target;
  document.getElementById("prompt").textContent =
    "Press a new combination to replace " + shown;
  save.textContent = "Save Shortcut";
  document.getElementById("remove").style.display = "";
  document.title = "Change Shortcut — Gretchen Flow";
});
