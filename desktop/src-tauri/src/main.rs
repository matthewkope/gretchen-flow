//! Gretchen Flow — menu-bar push-to-talk dictation.
//!
//! The menu bar ¿ shows the app state: quiet template glyph when idle,
//! bold glowing red while recording, amber while transcribing, with a ↓
//! suffix while the model downloads.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod config;
mod history;
mod inject;
mod lists;
mod model;
mod transcribe;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const TRAY_ID: &str = "gf";
const ICON_IDLE_DARK: &[u8] = include_bytes!("../icons/tray/idle.png");
const ICON_IDLE_LIGHT: &[u8] = include_bytes!("../icons/tray/idle-light.png");
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
    /// Idle badge theme: true = dark (white art on black), false = light.
    icon_dark: AtomicBool,
    /// The currently registered global shortcut (changeable from the menu).
    current_shortcut: Mutex<String>,
    cfg: config::Config,
    /// Full texts behind the tray's "Recent" items, newest first.
    history_items: Mutex<Vec<String>>,
}

/// Hotkey choices offered in the tray menu: (accelerator, display label).
const HOTKEY_CHOICES: &[(&str, &str)] = &[
    ("Ctrl+Alt+Space", "Control + Option + Space"),
    ("Cmd+Shift+Space", "Command + Shift + Space"),
    ("Ctrl+Alt+D", "Control + Option + D"),
    ("Cmd+Alt+G", "Command + Option + G"),
    ("F6", "F6"),
];

fn set_tray_state(app: &AppHandle, state: TrayState) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    // Dark theme: white Gretchen on a black badge (full color). Light theme:
    // the original silhouette as a macOS template image, recolored by the OS.
    let dark = app.state::<AppState>().icon_dark.load(Ordering::SeqCst);
    let (idle, idle_template) = if dark {
        (ICON_IDLE_DARK, false)
    } else {
        (ICON_IDLE_LIGHT, true)
    };
    let (bytes, template, title) = match state {
        TrayState::Idle => (idle, idle_template, None),
        TrayState::Downloading => (idle, idle_template, Some("↓")),
        TrayState::Recording => (ICON_RECORDING, false, None),
        TrayState::Transcribing => (ICON_TRANSCRIBING, false, None),
        TrayState::Error => (idle, idle_template, Some("✕")),
    };
    let _ = tray.set_icon(Image::from_bytes(bytes).ok());
    let _ = tray.set_icon_as_template(template);
    // Always set an explicit title: clearing with None doesn't reliably
    // remove the previous text on macOS.
    let _ = tray.set_title(Some(title.unwrap_or("")));
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
                history::append(&text);
                refresh_menu(&app);
            }
            Ok(_) => log::info!("no speech detected"),
            Err(e) => log::error!("transcription failed: {e}"),
        }
        set_tray_state(&app, TrayState::Idle);
    });
}

/// Menu action: switch the idle icon between dark and light, remember the
/// choice in the config file, and relabel the menu item.
fn toggle_icon_theme(app: &AppHandle) {
    let state = app.state::<AppState>();
    let dark = !state.icon_dark.load(Ordering::SeqCst);
    state.icon_dark.store(dark, Ordering::SeqCst);
    if !state.recording.load(Ordering::SeqCst) {
        set_tray_state(app, TrayState::Idle);
    }
    let mut cfg = config::Config::load();
    cfg.icon_theme = if dark { "dark".into() } else { "light".into() };
    cfg.save();
    log::info!("icon theme: {}", cfg.icon_theme);
    refresh_menu(app);
}

/// Switch the global hotkey, persist it, and update the menu. On failure the
/// previous hotkey stays registered.
fn set_hotkey(app: &AppHandle, accel: &str) -> Result<(), String> {
    let new_shortcut: Shortcut = accel
        .parse()
        .map_err(|e| format!("\"{accel}\" isn't a usable shortcut: {e}"))?;
    let state = app.state::<AppState>();
    let old = state.current_shortcut.lock().unwrap().clone();
    if old == accel {
        return Ok(());
    }
    if let Ok(old_shortcut) = old.parse::<Shortcut>() {
        let _ = app.global_shortcut().unregister(old_shortcut);
    }
    if let Err(e) = app
        .global_shortcut()
        .on_shortcut(new_shortcut, |app, _sc, event| {
            on_shortcut(app, event.state())
        })
    {
        if let Ok(old_shortcut) = old.parse::<Shortcut>() {
            let _ = app
                .global_shortcut()
                .on_shortcut(old_shortcut, |app, _sc, event| {
                    on_shortcut(app, event.state())
                });
        }
        return Err(format!("couldn't register \"{accel}\": {e}"));
    }
    *state.current_shortcut.lock().unwrap() = accel.to_string();
    let mut cfg = config::Config::load();
    cfg.shortcut = accel.to_string();
    cfg.save();
    log::info!("hotkey set to {accel}");
    refresh_menu(app);
    Ok(())
}

/// Open (or focus) the small "press your shortcut" recorder window.
fn open_hotkey_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("hotkey") {
        let _ = window.set_focus();
        return;
    }
    let result = tauri::WebviewWindowBuilder::new(
        app,
        "hotkey",
        tauri::WebviewUrl::App("hotkey.html".into()),
    )
    .title("Set Hotkey — Gretchen Flow")
    .inner_size(380.0, 230.0)
    .resizable(false)
    .always_on_top(true)
    .build();
    if let Err(e) = result {
        log::error!("couldn't open hotkey window: {e}");
    }
}

#[tauri::command]
fn apply_custom_hotkey(app: AppHandle, accel: String) -> Result<(), String> {
    set_hotkey(&app, &accel)?;
    if let Some(window) = app.get_webview_window("hotkey") {
        let _ = window.close();
    }
    Ok(())
}

#[tauri::command]
fn cancel_custom_hotkey(app: AppHandle) {
    if let Some(window) = app.get_webview_window("hotkey") {
        let _ = window.close();
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
    let current_shortcut = state.current_shortcut.lock().unwrap().clone();
    let status_label = format!(
        "Gretchen Flow — {} ({})",
        current_shortcut, state.cfg.hotkey_mode
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
        let hotkey_menu = Submenu::with_id(app, "hotkey-menu", "Hotkey", true)?;
        let mut current_listed = false;
        for (accel, label) in HOTKEY_CHOICES {
            let checked = *accel == current_shortcut;
            current_listed |= checked;
            hotkey_menu.append(&CheckMenuItem::with_id(
                app,
                format!("hotkey-{accel}"),
                *label,
                true,
                checked,
                None::<&str>,
            )?)?;
        }
        if !current_listed {
            // A custom shortcut — show it as the checked entry.
            hotkey_menu.append(&CheckMenuItem::with_id(
                app,
                format!("hotkey-{current_shortcut}"),
                &current_shortcut,
                true,
                true,
                None::<&str>,
            )?)?;
        }
        hotkey_menu.append(&PredefinedMenuItem::separator(app)?)?;
        hotkey_menu.append(&MenuItem::with_id(
            app,
            "record-hotkey",
            "Set Custom Hotkey…",
            true,
            None::<&str>,
        )?)?;
        menu.append(&hotkey_menu)?;

        let theme_label = if state.icon_dark.load(Ordering::SeqCst) {
            "Icon: Dark — switch to Light"
        } else {
            "Icon: Light — switch to Dark"
        };
        menu.append(&MenuItem::with_id(
            app,
            "theme",
            theme_label,
            true,
            None::<&str>,
        )?)?;
        menu.append(&PredefinedMenuItem::separator(app)?)?;
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
    if id == "theme" {
        toggle_icon_theme(app);
        return;
    }
    if id == "record-hotkey" {
        open_hotkey_window(app);
        return;
    }
    if let Some(accel) = id.strip_prefix("hotkey-") {
        if accel != "menu" {
            if let Err(e) = set_hotkey(app, accel) {
                log::error!("{e}");
            }
        }
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
        .invoke_handler(tauri::generate_handler![
            apply_custom_hotkey,
            cancel_custom_hotkey
        ])
        .manage(AppState {
            recorder: audio::Recorder::spawn(),
            engine: Arc::new(Mutex::new(None)),
            recording: AtomicBool::new(false),
            icon_dark: AtomicBool::new(cfg.icon_theme != "light"),
            current_shortcut: Mutex::new(cfg.shortcut.clone()),
            cfg,
            history_items: Mutex::new(Vec::new()),
        })
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let dark = app.state::<AppState>().icon_dark.load(Ordering::SeqCst);
            let initial_icon = if dark {
                ICON_IDLE_DARK
            } else {
                ICON_IDLE_LIGHT
            };
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(Image::from_bytes(initial_icon)?)
                .icon_as_template(!dark)
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
