//! Whisper inference via whisper-rs (whisper.cpp, Metal-accelerated on macOS),
//! plus pause-based punctuation: a silence of `pause_ms` between words becomes
//! a period, and the next word is capitalized. Smaller Whisper models often
//! skip punctuation entirely; this recovers sentence structure from how the
//! user actually spoke.

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct Engine {
    ctx: WhisperContext,
    language: Option<String>,
    pause_ms: i64,
}

struct Token {
    text: String,
    /// Start/end time in 10 ms units (whisper.cpp convention).
    t0: i64,
    t1: i64,
}

impl Engine {
    pub fn load(model_path: &str, language: &str, pause_ms: i64) -> Result<Self, String> {
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| format!("failed to load model: {e}"))?;
        let language = match language {
            "auto" | "" => None,
            l => Some(l.to_string()),
        };
        Ok(Self {
            ctx,
            language,
            pause_ms,
        })
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
        params.set_token_timestamps(self.pause_ms > 0);

        state.full(params, samples).map_err(|e| e.to_string())?;

        let n = state.full_n_segments().map_err(|e| e.to_string())?;

        if self.pause_ms == 0 {
            let mut text = String::new();
            for i in 0..n {
                let seg = state.full_get_segment_text(i).map_err(|e| e.to_string())?;
                text.push_str(seg.trim());
                text.push(' ');
            }
            return Ok(text.trim().to_string());
        }

        let mut tokens = Vec::new();
        for seg in 0..n {
            let n_tokens = state.full_n_tokens(seg).map_err(|e| e.to_string())?;
            for i in 0..n_tokens {
                let text = state
                    .full_get_token_text(seg, i)
                    .map_err(|e| e.to_string())?;
                // Skip special/marker tokens like <|en|> or [_BEG_].
                if text.starts_with("<|") || text.starts_with("[_") {
                    continue;
                }
                let data = state
                    .full_get_token_data(seg, i)
                    .map_err(|e| e.to_string())?;
                tokens.push(Token {
                    text,
                    t0: data.t0,
                    t1: data.t1,
                });
            }
        }

        let english = self.language.as_deref() == Some("en");
        Ok(build_text(&tokens, self.pause_ms, english))
    }
}

/// Join tokens into text, turning speech pauses into sentence breaks.
fn build_text(tokens: &[Token], pause_ms: i64, english: bool) -> String {
    let mut out = String::new();
    let mut last_t1: Option<i64> = None;
    let mut capitalize = true;

    for tok in tokens {
        if let Some(prev_end) = last_t1 {
            let gap_ms = (tok.t0 - prev_end) * 10;
            if gap_ms >= pause_ms {
                let trimmed = out.trim_end();
                if !trimmed.is_empty() && !trimmed.ends_with(['.', '!', '?', ',', ':', ';', '—'])
                {
                    out.truncate(trimmed.len());
                    out.push('.');
                    capitalize = true;
                } else if trimmed.ends_with([',', ':', ';']) {
                    // A pause after a comma still starts a new clause, not a
                    // new sentence — leave it as the model wrote it.
                }
            }
        }
        last_t1 = Some(tok.t1);

        if capitalize {
            out.push_str(&capitalize_first_alpha(&tok.text, &mut capitalize));
        } else {
            out.push_str(&tok.text);
        }
        if tok.text.trim_end().ends_with(['.', '!', '?']) {
            capitalize = true;
        }
    }

    let mut result = out.trim().to_string();
    if english {
        result = fix_standalone_i(&result);
    }
    if result.chars().last().is_some_and(|c| c.is_alphanumeric()) {
        result.push('.');
    }
    result
}

/// Uppercase the first alphabetic character; clear the flag once one is found.
fn capitalize_first_alpha(text: &str, capitalize: &mut bool) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        if *capitalize && c.is_alphabetic() {
            result.extend(c.to_uppercase());
            *capitalize = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// English nicety: standalone "i" (and contractions like "i'm") -> "I".
fn fix_standalone_i(text: &str) -> String {
    text.split(' ')
        .map(|word| {
            let rest = &word[word.len().min(1)..];
            let standalone =
                rest.is_empty() || rest.starts_with(['\'', '’', ',', '.', '!', '?', ';', ':']);
            if word.starts_with('i') && standalone {
                format!("I{rest}")
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(text: &str, t0: i64, t1: i64) -> Token {
        Token {
            text: text.into(),
            t0,
            t1,
        }
    }

    #[test]
    fn pause_becomes_period_and_capitalizes() {
        // 1.0 s gap between "world" and "this" (t in 10 ms units).
        let tokens = [
            tok(" hello", 0, 50),
            tok(" world", 50, 100),
            tok(" this", 200, 250),
            tok(" works", 250, 300),
        ];
        assert_eq!(build_text(&tokens, 700, true), "Hello world. This works.");
    }

    #[test]
    fn short_gap_does_not_break_sentence() {
        let tokens = [tok(" hello", 0, 50), tok(" world", 80, 130)];
        assert_eq!(build_text(&tokens, 700, true), "Hello world.");
    }

    #[test]
    fn existing_punctuation_is_kept() {
        let tokens = [
            tok(" great,", 0, 50),
            tok(" right", 200, 250),
            tok("?", 250, 251),
        ];
        assert_eq!(build_text(&tokens, 700, true), "Great, right?");
    }

    #[test]
    fn fixes_standalone_i() {
        let tokens = [
            tok(" i", 0, 10),
            tok(" think", 10, 40),
            tok(" i'm", 40, 70),
            tok(" right", 70, 100),
        ];
        assert_eq!(build_text(&tokens, 700, true), "I think I'm right.");
    }

    #[test]
    fn words_starting_with_i_untouched() {
        assert_eq!(fix_standalone_i("it is icy"), "it is icy");
        assert_eq!(fix_standalone_i("i, i. i"), "I, I. I");
    }

    #[test]
    fn pause_disabled_via_zero_is_handled_by_caller() {
        // build_text is only called when pause_ms > 0; still, a huge threshold
        // means no inserted periods.
        let tokens = [tok(" a", 0, 10), tok(" b", 500, 510)];
        assert_eq!(build_text(&tokens, 100_000, true), "A b.");
    }
}
