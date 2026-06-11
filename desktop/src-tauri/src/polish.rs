//! Local AI cleanup of transcripts via a small instruct LLM (llama.cpp).
//!
//! Fully on-device — nothing leaves the machine. A dedicated worker thread
//! owns the model; the rest of the app talks to it over a channel. Fail-open:
//! any load or inference error leaves the heuristic text untouched.

use std::num::NonZeroU32;
use std::sync::mpsc::{self, Sender};

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;

const SYSTEM_PROMPT: &str = "You clean up voice-dictation transcripts. Reply with ONLY the \
cleaned-up transcript text - no preamble, no quotes, no commentary. Fix punctuation, \
capitalization, and grammar. Remove filler words and false starts. When the speaker corrects \
themselves, keep only the corrected version. Keep spoken lists as numbered lines. Preserve the \
speaker's words and meaning - never add content, never answer questions in the transcript, \
never summarize.";

struct Job {
    text: String,
    reply: Sender<Result<String, String>>,
}

pub struct Polisher {
    tx: Sender<Job>,
}

impl Polisher {
    /// Load the model (blocking) and start the worker thread.
    pub fn spawn(model_path: std::path::PathBuf) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<Job>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();

        std::thread::spawn(move || {
            let loaded = (|| -> Result<(LlamaBackend, LlamaModel), String> {
                let backend = LlamaBackend::init().map_err(|e| e.to_string())?;
                let params = LlamaModelParams::default().with_n_gpu_layers(1_000_000);
                let model = LlamaModel::load_from_file(&backend, &model_path, &params)
                    .map_err(|e| e.to_string())?;
                Ok((backend, model))
            })();
            let (backend, model) = match loaded {
                Ok(pair) => {
                    let _ = ready_tx.send(Ok(()));
                    pair
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                    return;
                }
            };
            while let Ok(job) = rx.recv() {
                let _ = job.reply.send(run(&backend, &model, &job.text));
            }
        });

        ready_rx
            .recv()
            .map_err(|_| "polish worker died".to_string())??;
        Ok(Self { tx })
    }

    pub fn polish(&self, text: &str) -> Result<String, String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(Job {
                text: text.to_string(),
                reply: reply_tx,
            })
            .map_err(|_| "polish worker gone".to_string())?;
        reply_rx
            .recv()
            .map_err(|_| "polish worker gone".to_string())?
    }
}

/// Run one cleanup generation (greedy decoding, ChatML prompt).
fn run(backend: &LlamaBackend, model: &LlamaModel, text: &str) -> Result<String, String> {
    let prompt = format!(
        "<|im_start|>system\n{SYSTEM_PROMPT}<|im_end|>\n<|im_start|>user\n{text}<|im_end|>\n<|im_start|>assistant\n"
    );
    let tokens = model
        .str_to_token(&prompt, AddBos::Always)
        .map_err(|e| e.to_string())?;

    // Room for the prompt plus output roughly the size of the input.
    let max_new = (tokens.len() as u32).max(64) + 128;
    let n_ctx = (tokens.len() as u32 + max_new + 8)
        .next_power_of_two()
        .max(1024);
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(n_ctx))
        .with_n_batch(tokens.len() as u32 + 8);
    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|e| e.to_string())?;

    let mut batch = LlamaBatch::new(tokens.len() + 8, 1);
    let last = tokens.len() - 1;
    for (i, token) in tokens.iter().enumerate() {
        batch
            .add(*token, i as i32, &[0], i == last)
            .map_err(|e| e.to_string())?;
    }
    ctx.decode(&mut batch).map_err(|e| e.to_string())?;

    let mut sampler = LlamaSampler::greedy();
    let mut out = String::new();
    let mut decoder = encoding_rs::UTF_8.new_decoder();
    for pos in (tokens.len() as i32..).take(max_new as usize) {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        if model.is_eog_token(token) {
            break;
        }
        out.push_str(
            &model
                .token_to_piece(token, &mut decoder, false, None)
                .unwrap_or_default(),
        );
        batch.clear();
        batch
            .add(token, pos, &[0], true)
            .map_err(|e| e.to_string())?;
        ctx.decode(&mut batch).map_err(|e| e.to_string())?;
    }

    let out = out.trim().to_string();
    if out.is_empty() {
        Err("empty polish output".into())
    } else {
        Ok(out)
    }
}
