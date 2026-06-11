//! Gretchen Flow — menu-bar push-to-talk dictation.
//!
//! The menu bar ¿ shows the app state: quiet template glyph when idle,
//! bold glowing red while recording, amber while transcribing, with a ↓
//! suffix while the model downloads.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod config;
mod format_ai;
mod history;
mod inject;
mod lists;
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
    /// Full texts behind the tray's "Recent" items, newest first.
    history_items: Mutex<Vec<String>>,
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
                let text = ai_format_or_fallback(&state.cfg, text);
                log::info!("transcribed: {text}");
                if let Err(e) = inject::type_text(&text) {
                    log::error!("{e}");
                }
                history::append(&text);
                refresh_menu(&app);
            }
            Ok(_) => log::info!("no speech detected"),
            Err(e) => log::error!("transcription failed: {e}"),
        }
        set_tray_state(&app, TrayState::Idle);
    });
}

/// Run the AI formatting pass when enabled and a key is available;
/// otherwise (or on any error) keep the locally-formatted text.
fn ai_format_or_fallback(cfg: &config::Config, text: String) -> String {
    if !cfg.ai_format {
        return text;
    }
    let Some(key) = format_ai::api_key(cfg) else {
        return text;
    };
    match format_ai::format(cfg, &key, &text) {
        Ok(formatted) => formatted,
        Err(e) => {
            log::warn!("AI formatting failed, using local cleanup: {e}");
            text
        }
    }
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

const HISTORY_MENU_ITEMS: usize = 5;

/// Rebuild the tray menu, including the latest history entries.
fn refresh_menu(app: &AppHandle) {
    let state = app.state::<AppState>();
    let recent = history::recent(HISTORY_MENU_ITEMS);
    let status_label = format!(
        "Gretchen Flow — {} ({})",
        state.cfg.shortcut, state.cfg.hotkey_mode
    );
    let result = (|| -> tauri::Result<()> {
        let menu = Menu::new(app)?;
        menu.append(&MenuItem::with_id(
            app,
            "status",
            &status_label,
            false,
            None::<&str>,
        )?)?;
        menu.append(&PredefinedMenuItem::separator(app)?)?;
        if !recent.is_empty() {
            menu.append(&MenuItem::with_id(
                app,
                "hist-header",
                "Recent — click to type again",
                false,
                None::<&str>,
            )?)?;
            for (i, text) in recent.iter().enumerate() {
                let label: String = if text.chars().count() > 45 {
                    format!("{}…", text.chars().take(45).collect::<String>())
                } else {
                    text.clone()
                };
                menu.append(&MenuItem::with_id(
                    app,
                    format!("hist-{i}"),
                    label,
                    true,
                    None::<&str>,
                )?)?;
            }
            menu.append(&PredefinedMenuItem::separator(app)?)?;
        }
        menu.append(&MenuItem::with_id(
            app,
            "quit",
            "Quit Gretchen Flow",
            true,
            None::<&str>,
        )?)?;
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            tray.set_menu(Some(menu))?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        log::error!("menu refresh failed: {e}");
    }
    *state.history_items.lock().unwrap() = recent;
}

fn on_menu_event(app: &AppHandle, id: &str) {
    if id == "quit" {
        app.exit(0);
        return;
    }
    let Some(idx) = id
        .strip_prefix("hist-")
        .and_then(|s| s.parse::<usize>().ok())
    else {
        return;
    };
    let text = {
        let state = app.state::<AppState>();
        let items = state.history_items.lock().unwrap();
        items.get(idx).cloned()
    };
    if let Some(text) = text {
        std::thread::spawn(move || {
            // Give the menu a beat to close and focus to return to the app
            // the user was in.
            std::thread::sleep(std::time::Duration::from_millis(300));
            if let Err(e) = inject::type_text(&text) {
                log::error!("{e}");
            }
        });
    }
}

fn load_engine_async(app: AppHandle) {
    std::thread::spawn(move || {
        set_tray_state(&app, TrayState::Downloading);
        let state = app.state::<AppState>();
        let cfg = state.cfg.clone();
        let loaded = model::ensure_model(&cfg.model)
            .and_then(|path| transcribe::Engine::load(&path.to_string_lossy(), &cfg));
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
            history_items: Mutex::new(Vec::new()),
        })
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            TrayIconBuilder::with_id(TRAY_ID)
                .icon(Image::from_bytes(ICON_IDLE)?)
                .icon_as_template(true)
                .title("↓")
                .on_menu_event(|app, event| on_menu_event(app, event.id().as_ref()))
                .build(app)?;
            refresh_menu(app.handle());

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
