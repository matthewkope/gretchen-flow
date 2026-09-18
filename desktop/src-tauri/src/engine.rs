use crate::{config::Config, parakeet, transcribe};

pub enum Engine {
    Whisper(transcribe::Engine),
    Parakeet(parakeet::Engine),
}
impl Engine {
    pub fn load(path: &str, cfg: &Config) -> Result<Self, String> {
        if cfg.model == crate::config::DEFAULT_MODEL {
            parakeet::Engine::load(path, cfg).map(Self::Parakeet)
        } else {
            transcribe::Engine::load(path, cfg).map(Self::Whisper)
        }
    }
    pub fn transcribe(&self, samples: &[f32]) -> Result<String, String> {
        match self {
            Self::Whisper(engine) => engine.transcribe(samples),
            Self::Parakeet(engine) => engine.transcribe(samples),
        }
    }
}
