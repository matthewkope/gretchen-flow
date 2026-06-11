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

/// Ensure the Whisper model is available locally. Absolute paths are used
/// as-is (user-supplied model files); names are downloaded from the
/// whisper.cpp collection if needed.
pub fn ensure_model(name: &str) -> Result<PathBuf, String> {
    if name.starts_with('/') {
        let path = PathBuf::from(name);
        return if path.exists() {
            Ok(path)
        } else {
            Err(format!("model file not found: {name}"))
        };
    }
    let path = model_path(name);
    if path.exists() {
        return Ok(path);
    }
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;

    let url = format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{name}.bin");
    log::info!("downloading model {name} from {url}");

    let resp = ureq::get(&url)
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let tmp = path.with_extension("bin.part");
    let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    std::io::copy(&mut resp.into_reader(), &mut file).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    log::info!("model saved to {}", path.display());
    Ok(path)
}
