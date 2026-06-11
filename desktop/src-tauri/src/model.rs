//! Download ggml Whisper models from the ggerganov/whisper.cpp Hugging Face repo
//! into `~/.cache/gretchen-flow/models/`.

use std::fs;
use std::path::PathBuf;

pub fn model_path(name: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache/gretchen-flow/models")
        .join(format!("ggml-{name}.bin"))
}

/// Ensure the Whisper model file exists locally, downloading it if needed.
pub fn ensure_model(name: &str) -> Result<PathBuf, String> {
    let url = format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{name}.bin");
    ensure_file(&url, &format!("ggml-{name}.bin"))
}

/// Ensure an arbitrary model file (by URL) exists in the cache directory.
pub fn ensure_file(url: &str, filename: &str) -> Result<PathBuf, String> {
    let path = model_path("x").parent().unwrap().join(filename);
    if path.exists() {
        return Ok(path);
    }
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    log::info!("downloading {filename} from {url}");

    let resp = ureq::get(url)
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let tmp = path.with_extension("part");
    let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    std::io::copy(&mut resp.into_reader(), &mut file).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    log::info!("saved to {}", path.display());
    Ok(path)
}
