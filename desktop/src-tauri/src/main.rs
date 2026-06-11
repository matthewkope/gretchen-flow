//! Gretchen Flow — menu-bar push-to-talk dictation.
//!
//! Menu bar shows the app state: ¿ idle, ↓ downloading model, ● recording,
//! … transcribing.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod config;
mod inject;
mod model;
mod transcribe;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const TRAY_ID: &str = "gf";
const IDLE: &str = "¿";
const DOWNLOADING: &str = "↓¿";
const RECORDING: &str = "●";
const TRANSCRIBING: &str = "…";

struct AppState {
    recorder: audio::Recorder,
    engine: Arc<Mutex<Option<transcribe::Engine>>>,
    recording: AtomicBool,
    cfg: config::Config,
}

fn set_tray_title(app: &AppHandle, title: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(Some(title));
    }
}

fn start_recording(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.engine.lock().unwrap().is_none() {
        log::warn!("model not loaded yet; ignoring hotkey");
        return;
    }
    state.recording.store(true, Ordering::SeqCst);
    state.recorder.start();
    set_tray_title(app, RECORDING);
}

fn stop_and_transcribe(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.recording.store(false, Ordering::SeqCst);
    let recording = state.recorder.stop();
    let seconds = recording.samples.len() as f32 / recording.sample_rate as f32;
    if seconds < 0.3 {
        log::info!("recording too short ({seconds:.2}s), ignored");
        set_tray_title(app, IDLE);
        return;
    }
    set_tray_title(app, TRANSCRIBING);

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
        set_tray_title(&app, IDLE);
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
        set_tray_title(&app, DOWNLOADING);
        let state = app.state::<AppState>();
        let cfg = state.cfg.clone();
        let loaded = model::ensure_model(&cfg.model)
            .and_then(|path| transcribe::Engine::load(&path.to_string_lossy(), &cfg.language));
        match loaded {
            Ok(engine) => {
                *state.engine.lock().unwrap() = Some(engine);
                log::info!("engine ready (model: {})", cfg.model);
                set_tray_title(&app, IDLE);
            }
            Err(e) => {
                log::error!("engine load failed: {e}");
                set_tray_title(&app, "✕¿");
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
            let menu = Menu::with_items(
                app,
                &[&status, &PredefinedMenuItem::separator(app)?, &quit],
            )?;
            TrayIconBuilder::with_id(TRAY_ID)
                .title(DOWNLOADING)
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
