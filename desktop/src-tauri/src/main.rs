//! Gretchen Flow — menu-bar push-to-talk dictation.
//!
//! The menu bar ¿ shows the app state: quiet template glyph when idle,
//! bold glowing red while recording, amber while transcribing, with a ↓
//! suffix while the model downloads.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod config;
mod inject;
mod model;
mod transcribe;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const TRAY_ID: &str = "gf";
const ICON_IDLE: &[u8] = include_bytes!("../icons/tray/idle.png");
const ICON_RECORDING: &[u8] = include_bytes!("../icons/tray/recording.png");
const ICON_TRANSCRIBING: &[u8] = include_bytes!("../icons/tray/transcribing.png");

#[derive(Clone, Copy)]
enum TrayState {
    Downloading,
    Idle,
    Recording,
    Transcribing,
    Error,
}

struct AppState {
    recorder: audio::Recorder,
    engine: Arc<Mutex<Option<transcribe::Engine>>>,
    recording: AtomicBool,
    cfg: config::Config,
}

fn set_tray_state(app: &AppHandle, state: TrayState) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    // Template icons are recolored by macOS for light/dark menu bars; the
    // recording/transcribing icons keep their own colors so they stand out.
    let (bytes, template, title) = match state {
        TrayState::Idle => (ICON_IDLE, true, None),
        TrayState::Downloading => (ICON_IDLE, true, Some("↓")),
        TrayState::Recording => (ICON_RECORDING, false, None),
        TrayState::Transcribing => (ICON_TRANSCRIBING, false, None),
        TrayState::Error => (ICON_IDLE, true, Some("✕")),
    };
    let _ = tray.set_icon(Image::from_bytes(bytes).ok());
    let _ = tray.set_icon_as_template(template);
    let _ = tray.set_title(title);
}

fn start_recording(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.engine.lock().unwrap().is_none() {
        log::warn!("model not loaded yet; ignoring hotkey");
        return;
    }
    state.recording.store(true, Ordering::SeqCst);
    state.recorder.start();
    set_tray_state(app, TrayState::Recording);
}

fn stop_and_transcribe(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.recording.store(false, Ordering::SeqCst);
    let recording = state.recorder.stop();
    let seconds = recording.samples.len() as f32 / recording.sample_rate as f32;
    if seconds < 0.3 {
        log::info!("recording too short ({seconds:.2}s), ignored");
        set_tray_state(app, TrayState::Idle);
        return;
    }
    set_tray_state(app, TrayState::Transcribing);

    let app = app.clone();
    std::thread::spawn(move || {
        let samples = audio::resample_to_16k(&recording);
        let state = app.state::<AppState>();
        let result = {
            let guard = state.engine.lock().unwrap();
            match guard.as_ref() {
                Some(engine) => engine.transcribe(&samples),
                None => Err("model not loaded".into()),
            }
        };
        match result {
            Ok(text) if !text.is_empty() => {
                log::info!("transcribed: {text}");
                if let Err(e) = inject::type_text(&text) {
                    log::error!("{e}");
                }
            }
            Ok(_) => log::info!("no speech detected"),
            Err(e) => log::error!("transcription failed: {e}"),
        }
        set_tray_state(&app, TrayState::Idle);
    });
}

fn on_shortcut(app: &AppHandle, state_event: ShortcutState) {
    let state = app.state::<AppState>();
    let hold_mode = state.cfg.hotkey_mode == "hold";
    let recording = state.recording.load(Ordering::SeqCst);

    match state_event {
        ShortcutState::Pressed => {
            if hold_mode {
                if !recording {
                    start_recording(app);
                }
            } else if recording {
                stop_and_transcribe(app);
            } else {
                start_recording(app);
            }
        }
        ShortcutState::Released => {
            if hold_mode && recording {
                stop_and_transcribe(app);
            }
        }
    }
}

fn load_engine_async(app: AppHandle) {
    std::thread::spawn(move || {
        set_tray_state(&app, TrayState::Downloading);
        let state = app.state::<AppState>();
        let cfg = state.cfg.clone();
        let loaded = model::ensure_model(&cfg.model).and_then(|path| {
            transcribe::Engine::load(
                &path.to_string_lossy(),
                &cfg.language,
                cfg.pause_punctuation_ms as i64,
            )
        });
        match loaded {
            Ok(engine) => {
                *state.engine.lock().unwrap() = Some(engine);
                log::info!("engine ready (model: {})", cfg.model);
                set_tray_state(&app, TrayState::Idle);
            }
            Err(e) => {
                log::error!("engine load failed: {e}");
                set_tray_state(&app, TrayState::Error);
            }
        }
    });
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cfg = config::Config::load();
    log::info!(
        "Gretchen Flow starting (model: {}, shortcut: {}, mode: {})",
        cfg.model,
        cfg.shortcut,
        cfg.hotkey_mode
    );

    let shortcut: Shortcut = cfg
        .shortcut
        .parse()
        .expect("invalid shortcut in config.json");

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            recorder: audio::Recorder::spawn(),
            engine: Arc::new(Mutex::new(None)),
            recording: AtomicBool::new(false),
            cfg,
        })
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let state = app.state::<AppState>();
            let status_label = format!(
                "Gretchen Flow — {} ({})",
                state.cfg.shortcut, state.cfg.hotkey_mode
            );
            let status = MenuItem::with_id(app, "status", status_label, false, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Gretchen Flow", true, None::<&str>)?;
            let menu =
                Menu::with_items(app, &[&status, &PredefinedMenuItem::separator(app)?, &quit])?;
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(Image::from_bytes(ICON_IDLE)?)
                .icon_as_template(true)
                .title("↓")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;

            app.global_shortcut()
                .on_shortcut(shortcut, |app, _sc, event| on_shortcut(app, event.state()))?;

            load_engine_async(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Gretchen Flow")
        .run(|_app, event| {
            // Keep running with zero windows; only exit via the Quit menu item.
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
