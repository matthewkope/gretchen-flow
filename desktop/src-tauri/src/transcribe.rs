//! Whisper inference via whisper-rs (whisper.cpp, Metal-accelerated on macOS).

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct Engine {
    ctx: WhisperContext,
    language: Option<String>,
}

impl Engine {
    pub fn load(model_path: &str, language: &str) -> Result<Self, String> {
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| format!("failed to load model: {e}"))?;
        let language = match language {
            "auto" | "" => None,
            l => Some(l.to_string()),
        };
        Ok(Self { ctx, language })
    }

    /// Transcribe 16 kHz mono f32 PCM.
    pub fn transcribe(&self, samples: &[f32]) -> Result<String, String> {
        let mut state = self.ctx.create_state().map_err(|e| e.to_string())?;
        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: 5,
            patience: -1.0,
        });
        params.set_language(self.language.as_deref());
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);

        state.full(params, samples).map_err(|e| e.to_string())?;

        let n = state.full_n_segments().map_err(|e| e.to_string())?;
        let mut text = String::new();
        for i in 0..n {
            let seg = state.full_get_segment_text(i).map_err(|e| e.to_string())?;
            text.push_str(seg.trim());
            text.push(' ');
        }
        Ok(text.trim().to_string())
    }
}
