const { invoke } = window.__TAURI__.core;
const status = document.getElementById("status");
const download = document.getElementById("download");
// Permission buttons open the matching Privacy pane.
for (const b of document.querySelectorAll(".perm")) {
  b.onclick = () => {
    invoke("open_privacy", { pane: b.dataset.pane });
    if (b.dataset.pane === "microphone") invoke("prime_microphone");
  };
}
download.onclick = () => {
  status.textContent = "Downloading… you can close this window; the menu bar icon shows ↓ while it works.";
  download.disabled = true;
  invoke("download_recommended_model");
  // Surface the permission prompts as part of first-run setup.
  invoke("prime_microphone");
  invoke("open_privacy", { pane: "accessibility" });
};
document.getElementById("close").onclick = () => invoke("close_setup");
