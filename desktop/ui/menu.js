const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

function row(opts) {
  const el = document.createElement("div");
  el.className = "item" + (opts.active ? " active" : "");
  const bullet = document.createElement("span");
  bullet.className = "bullet";
  // Selected/active rows are marked with a sideways triangle.
  bullet.textContent = opts.active ? "▸" : "•";
  const label = document.createElement("span");
  label.className = "label";
  label.textContent = opts.label;
  el.appendChild(bullet);
  el.appendChild(label);
  if (opts.note) {
    const note = document.createElement("span");
    note.className = "note";
    note.textContent = opts.note;
    el.appendChild(note);
  }
  el.onclick = opts.onclick;
  return el;
}

async function render() {
  const s = await invoke("menu_state");
  const status = document.getElementById("status");
  status.textContent = s.status;
  status.className = "status" + (s.has_engine ? "" : " warn");

  const models = document.getElementById("models");
  models.innerHTML = "";
  for (const m of s.models) {
    models.appendChild(row({
      label: m.label, active: m.active, note: m.note,
      onclick: () => invoke("menu_choose_model", { name: m.id }),
    }));
  }
  if (s.custom_model) {
    models.appendChild(row({ label: s.custom_model, active: true, note: "", onclick: () => {} }));
  }

  const hotkeys = document.getElementById("hotkeys");
  hotkeys.innerHTML = "";
  const removable = s.shortcuts.length > 1;
  for (const accel of s.shortcuts) {
    const label = accel === "Fn" ? "Fn  (🌐 Globe key)" : accel;
    const el = document.createElement("div");
    el.className = "item active";
    const bullet = document.createElement("span");
    bullet.className = "bullet"; bullet.textContent = "•";
    const lab = document.createElement("span");
    lab.className = "label"; lab.textContent = label;
    el.appendChild(bullet); el.appendChild(lab);
    // Click the row to re-record this shortcut as anything you like.
    el.onclick = () => invoke("menu_change_hotkey", { accel }).catch(() => {});
    if (removable) {
      const note = document.createElement("span");
      note.className = "note"; note.textContent = "remove";
      note.onclick = (ev) => {
        ev.stopPropagation();
        invoke("menu_remove_hotkey", { accel }).catch(() => {});
      };
      el.appendChild(note);
    }
    hotkeys.appendChild(el);
  }
  // "Add shortcut" is disabled once the maximum is reached.
  const add = document.getElementById("hotkey-record");
  add.classList.toggle("disabled", !s.can_add_shortcut);
  add.querySelector(".label").textContent =
    s.can_add_shortcut ? "add shortcut…" : "max 3 shortcuts";

  const recent = document.getElementById("recent");
  recent.innerHTML = "";
  if (!s.recent.length) {
    const e = document.createElement("div");
    e.className = "empty";
    e.textContent = "no dictations yet";
    recent.appendChild(e);
  } else {
    s.recent.forEach((text, i) => {
      const short = text.length > 40 ? text.slice(0, 40) + "…" : text;
      recent.appendChild(row({
        label: short, active: false, note: "copy",
        onclick: (e) => {
          invoke("menu_copy_recent", { index: i });
          const n = e.currentTarget.querySelector(".note");
          if (n) { n.textContent = "copied!"; setTimeout(() => { n.textContent = "copy"; }, 1000); }
        },
      }));
    });
  }
  // Offer "clear history" only when there's something to clear.
  document.getElementById("recent-clear").style.display =
    s.recent.length ? "" : "none";
}

document.getElementById("model-file").onclick = () => invoke("menu_model_from_file");
document.getElementById("model-custom").onclick = () => invoke("open_models_page");
document.getElementById("hotkey-record").onclick = (e) => {
  if (e.currentTarget.classList.contains("disabled")) return;
  invoke("menu_record_hotkey");
};
document.getElementById("setup").onclick = () => invoke("menu_open_setup");
document.getElementById("reload").onclick = () => invoke("menu_reload_config");
document.getElementById("recent-clear").onclick = () => invoke("menu_clear_history");
document.getElementById("quit").onclick = () => invoke("menu_quit");

listen("menu-updated", render);
render();
