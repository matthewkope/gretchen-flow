//! Microphone capture on a dedicated thread.
//!
//! cpal's `Stream` is not `Send`, so a worker thread owns it and the rest of
//! the app talks to it over a channel. The stream only exists while recording,
//! so the macOS mic indicator is accurate.

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};

pub struct Recording {
    /// Mono PCM at `sample_rate`.
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

enum Cmd {
    Start,
    Stop(Sender<Recording>),
}

pub struct Recorder {
    tx: Sender<Cmd>,
}

impl Recorder {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<Cmd>();
        std::thread::spawn(move || {
            let mut stream: Option<cpal::Stream> = None;
            let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
            let mut rate = 16_000u32;

            while let Ok(cmd) = rx.recv() {
                match cmd {
                    Cmd::Start => {
                        if stream.is_some() {
                            continue;
                        }
                        buffer.lock().unwrap().clear();
                        match build_stream(Arc::clone(&buffer)) {
                            Ok((s, r)) => {
                                rate = r;
                                stream = Some(s);
                            }
                            Err(e) => log::error!("failed to open microphone: {e}"),
                        }
                    }
                    Cmd::Stop(reply) => {
                        stream = None; // drop closes the input stream
                        let samples = std::mem::take(&mut *buffer.lock().unwrap());
                        let _ = reply.send(Recording {
                            samples,
                            sample_rate: rate,
                        });
                    }
                }
            }
        });
        Self { tx }
    }

    pub fn start(&self) {
        let _ = self.tx.send(Cmd::Start);
    }

    pub fn stop(&self) -> Recording {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self.tx.send(Cmd::Stop(reply_tx));
        reply_rx.recv().unwrap_or(Recording {
            samples: Vec::new(),
            sample_rate: 16_000,
        })
    }
}

fn build_stream(
    buffer: Arc<Mutex<Vec<f32>>>,
) -> Result<(cpal::Stream, u32), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("no default input device")?;
    let name = device.name().unwrap_or_else(|_| "<unknown>".into());
    let config = device.default_input_config()?;
    let rate = config.sample_rate().0;
    let channels = config.channels() as usize;
    let format = config.sample_format();
    log::info!("opening input '{name}' rate={rate} ch={channels} fmt={format:?}");

    let cfg: cpal::StreamConfig = config.into();
    // Capture in whatever native sample format the device reports, converting
    // each frame to mono f32. Hardcoding f32 broke on devices (e.g. the
    // built-in mic) whose native format is integer PCM.
    let stream = match format {
        SampleFormat::F32 => build_typed::<f32>(&device, &cfg, channels, buffer)?,
        SampleFormat::I16 => build_typed::<i16>(&device, &cfg, channels, buffer)?,
        SampleFormat::U16 => build_typed::<u16>(&device, &cfg, channels, buffer)?,
        SampleFormat::I8 => build_typed::<i8>(&device, &cfg, channels, buffer)?,
        SampleFormat::I32 => build_typed::<i32>(&device, &cfg, channels, buffer)?,
        SampleFormat::U8 => build_typed::<u8>(&device, &cfg, channels, buffer)?,
        other => {
            return Err(format!("unsupported input sample format {other:?}").into());
        }
    };
    stream.play()?;
    Ok((stream, rate))
}

/// Build an input stream for a concrete sample type, downmixing interleaved
/// frames to mono f32 into `buffer`.
fn build_typed<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    buffer: Arc<Mutex<Vec<f32>>>,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            let mut buf = buffer.lock().unwrap();
            for frame in data.chunks(channels) {
                let sum: f32 = frame.iter().map(|s| f32::from_sample(*s)).sum();
                buf.push(sum / channels as f32);
            }
        },
        |err| log::error!("audio stream error: {err}"),
        None,
    )
}

/// Linear resample to 16 kHz mono for Whisper.
pub fn resample_to_16k(rec: &Recording) -> Vec<f32> {
    const TARGET: u32 = 16_000;
    if rec.sample_rate == TARGET || rec.samples.is_empty() {
        return rec.samples.clone();
    }
    let ratio = rec.sample_rate as f64 / TARGET as f64;
    let out_len = (rec.samples.len() as f64 / ratio) as usize;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let idx = pos as usize;
            let frac = (pos - idx as f64) as f32;
            let a = rec.samples[idx.min(rec.samples.len() - 1)];
            let b = rec.samples[(idx + 1).min(rec.samples.len() - 1)];
            a + (b - a) * frac
        })
        .collect()
}
