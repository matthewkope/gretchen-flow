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
const ICON_APP: &[u8] = include_bytes!("../icons/icon.png");
const ICON_IDLE_DARK: &[u8] = include_bytes!("../icons/tray/idle.png");
const ICON_IDLE_LIGHT: &[u8] = include_bytes!("../icons/tray/idle-light.png");
const ICON_RECORDING: &[u8] = include_bytes!("../icons/tray/recording.png");
const ICON_TRANSCRIBING: &[u8] = include_bytes!("../icons/tray/transcribing.png");

#[derive(Clone, Copy, PartialEq)]
enum TrayState {
    Downloading,
    Idle,
    Recording,
    Transcribing,
    NeedsModel,
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
    /// True while the Fn/Globe key is the active hotkey.
    fn_hotkey: AtomicBool,
    /// The active Whisper model name (changeable from the menu).
    current_model: Mutex<String>,
    /// Live config; replaced wholesale by "Reload Config".
    cfg: Mutex<config::Config>,
    /// Full texts behind the tray's "Recent" items, newest first.
    history_items: Mutex<Vec<String>>,
}

/// Special hotkey value: the Fn/Globe key, watched by a low-level listener
/// because macOS can't register it as a normal shortcut.
const FN_HOTKEY: &str = "Fn";

/// The model recommended to first-time users (best accuracy/size balance).
const RECOMMENDED_MODEL: &str = "large-v3-turbo-q5_0";

/// Whisper model choices offered in the tray menu: (ggml name, display label).
const MODEL_CHOICES: &[(&str, &str)] = &[
    (
        "large-v3-turbo-q5_0",
        "Large v3 Turbo quantized — 547 MB (default)",
    ),
    ("large-v3-turbo", "Large v3 Turbo — 1.6 GB, max accuracy"),
    ("small", "Small — 466 MB, lighter"),
    ("base", "Base — 142 MB, fastest"),
];

/// Hotkey choices offered in the tray menu: (accelerator, display label).
const HOTKEY_CHOICES: &[(&str, &str)] = &[
    ("Ctrl+Alt+Space", "Control + Option + Space"),
    ("Cmd+Shift+Space", "Command + Shift + Space"),
    ("Ctrl+Alt+D", "Control + Option + D"),
    ("Cmd+Alt+G", "Command + Option + G"),
    ("F6", "F6"),
    ("Fn", "Fn  (🌐 Globe key)"),
];

/// Apply a tray state on the main thread (AppKit UI is not thread-safe).
fn set_tray_state(app: &AppHandle, state: TrayState) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || set_tray_state_on_main(&handle, state));
}

thread_local! {
    /// Last applied (state, dark) — skip no-op tray updates so an open
    /// dropdown isn't disturbed by redundant icon/title writes.
    static LAST_TRAY: std::cell::Cell<Option<(TrayState, bool)>> =
        const { std::cell::Cell::new(None) };
}

fn set_tray_state_on_main(app: &AppHandle, state: TrayState) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    // Dark theme: white Gretchen on a black badge (full color). Light theme:
    // the original silhouette as a macOS template image, recolored by the OS.
    let dark = app.state::<AppState>().icon_dark.load(Ordering::SeqCst);
    if LAST_TRAY.with(|c| c.get()) == Some((state, dark)) {
        return;
    }
    LAST_TRAY.with(|c| c.set(Some((state, dark))));
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
        TrayState::NeedsModel => (idle, idle_template, Some("!")),
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
        log::warn!("no model loaded; opening setup");
        show_setup(app);
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
        let mut samples = audio::resample_to_16k(&recording);
        // Whisper rejects clips under 1 s — pad short ones with silence.
        const MIN_SAMPLES: usize = 17_600; // 1.1 s at 16 kHz
        if samples.len() < MIN_SAMPLES {
            samples.resize(MIN_SAMPLES, 0.0);
        }
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
/// previous hotkey stays registered. The special value "Fn" switches to the
/// low-level Fn/Globe listener instead of a registered shortcut.
fn set_hotkey(app: &AppHandle, accel: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let old = state.current_shortcut.lock().unwrap().clone();
    if old == accel {
        return Ok(());
    }

    if accel == FN_HOTKEY {
        if let Ok(old_shortcut) = old.parse::<Shortcut>() {
            let _ = app.global_shortcut().unregister(old_shortcut);
        }
        state.fn_hotkey.store(true, Ordering::SeqCst);
    } else {
        let new_shortcut: Shortcut = accel
            .parse()
            .map_err(|e| format!("\"{accel}\" isn't a usable shortcut: {e}"))?;
        if old == FN_HOTKEY {
            state.fn_hotkey.store(false, Ordering::SeqCst);
        } else if let Ok(old_shortcut) = old.parse::<Shortcut>() {
            let _ = app.global_shortcut().unregister(old_shortcut);
        }
        if let Err(e) = app
            .global_shortcut()
            .on_shortcut(new_shortcut, |app, _sc, event| {
                on_shortcut(app, event.state())
            })
        {
            // Restore whatever was active before.
            if old == FN_HOTKEY {
                state.fn_hotkey.store(true, Ordering::SeqCst);
            } else if let Ok(old_shortcut) = old.parse::<Shortcut>() {
                let _ = app
                    .global_shortcut()
                    .on_shortcut(old_shortcut, |app, _sc, event| {
                        on_shortcut(app, event.state())
                    });
            }
            return Err(format!("couldn't register \"{accel}\": {e}"));
        }
    }

    *state.current_shortcut.lock().unwrap() = accel.to_string();
    let mut cfg = config::Config::load();
    cfg.shortcut = accel.to_string();
    cfg.save();
    log::info!("hotkey set to {accel}");
    refresh_menu(app);
    Ok(())
}

/// Watch the Fn/Globe key globally (it can't be a registered shortcut).
/// A minimal CGEventTap on flagsChanged events only — no keyboard-layout
/// lookups, so it's safe off the main thread. Always running; only acts
/// while `fn_hotkey` is set.
fn spawn_fn_listener(app: AppHandle) {
    use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
    use core_graphics::event::{
        CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
        CGEventType,
    };
    use std::cell::Cell;

    std::thread::spawn(move || {
        let fn_was_down = Cell::new(false);
        let tap = CGEventTap::new(
            CGEventTapLocation::Session,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![CGEventType::FlagsChanged],
            move |_proxy, _etype, event| {
                let down = event
                    .get_flags()
                    .contains(CGEventFlags::CGEventFlagSecondaryFn);
                if down != fn_was_down.get() {
                    fn_was_down.set(down);
                    if app.state::<AppState>().fn_hotkey.load(Ordering::SeqCst) {
                        let shortcut_state = if down {
                            ShortcutState::Pressed
                        } else {
                            ShortcutState::Released
                        };
                        on_shortcut(&app, shortcut_state);
                    }
                }
                None
            },
        );
        let Ok(tap) = tap else {
            log::error!("couldn't create Fn event tap (check Input Monitoring permission)");
            return;
        };
        let Ok(source) = tap.mach_port.create_runloop_source(0) else {
            log::error!("couldn't create run loop source for the Fn tap");
            return;
        };
        let run_loop = CFRunLoop::get_current();
        unsafe {
            run_loop.add_source(&source, kCFRunLoopCommonModes);
        }
        tap.enable();
        CFRunLoop::run_current();
    });
}

/// Menu action: switch the Whisper model. Downloads it if needed and swaps
/// the engine in the background; on failure the previous model stays active.
fn set_model(app: &AppHandle, name: &str) {
    let state = app.state::<AppState>();
    let previous = {
        let mut current = state.current_model.lock().unwrap();
        // Skip only if this model is already active and loaded; otherwise
        // (re)load so a missing/never-downloaded model still gets fetched.
        if *current == name && state.engine.lock().unwrap().is_some() {
            return;
        }
        let previous = current.clone();
        *current = name.to_string();
        previous
    };
    let mut cfg = config::Config::load();
    cfg.model = name.to_string();
    cfg.save();
    refresh_menu(app);

    let app = app.clone();
    let name = name.to_string();
    std::thread::spawn(move || {
        set_tray_state(&app, TrayState::Downloading);
        let cfg = config::Config::load();
        let loaded = model::ensure_model(&name)
            .and_then(|path| transcribe::Engine::load(&path.to_string_lossy(), &cfg));
        let state = app.state::<AppState>();
        match loaded {
            Ok(engine) => {
                *state.engine.lock().unwrap() = Some(engine);
                log::info!("switched model to {name}");
            }
            Err(e) => {
                log::error!("model switch to {name} failed, keeping {previous}: {e}");
                *state.current_model.lock().unwrap() = previous.clone();
                let mut cfg = config::Config::load();
                cfg.model = previous;
                cfg.save();
            }
        }
        // Reflect the result: ready, or still no model.
        let idle = if state.engine.lock().unwrap().is_some() {
            TrayState::Idle
        } else {
            TrayState::NeedsModel
        };
        set_tray_state(&app, idle);
        refresh_menu(&app);
    });
}

/// Open (or focus) the small "press your shortcut" recorder window.
fn open_hotkey_window(app: &AppHandle) {
    activate_app();
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

/// Activate the app so panels (file dialogs) center on screen — without
/// this, an accessory app's dialogs appear in odd positions.
fn activate_app() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;
    if let Some(mtm) = MainThreadMarker::new() {
        let ns_app = NSApplication::sharedApplication(mtm);
        #[allow(deprecated)]
        ns_app.activateIgnoringOtherApps(true);
    }
}

/// Set the Dock icon to the Gretchen artwork at runtime, so it's never the
/// generic executable icon (e.g. in dev builds, or before the bundle icon
/// is associated). Main thread only.
fn set_dock_icon() {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let data = NSData::with_bytes(ICON_APP);
    if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
        let ns_app = NSApplication::sharedApplication(mtm);
        unsafe { ns_app.setApplicationIconImage(Some(&image)) };
    }
}

/// Native, screen-centered open panel for choosing a Whisper model file from
/// anywhere on disk. Runs on the main thread (where menu events fire) so the
/// modal panel can present and center correctly for an accessory app.
fn pick_model_file(app: &AppHandle) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
    use objc2_foundation::{NSArray, NSString};

    activate_app();
    let Some(mtm) = MainThreadMarker::new() else {
        log::error!("model picker must run on the main thread");
        return;
    };

    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseFiles(true);
    panel.setCanChooseDirectories(false);
    panel.setAllowsMultipleSelection(false);
    let exts =
        NSArray::from_retained_slice(&[NSString::from_str("bin"), NSString::from_str("gguf")]);
    #[allow(deprecated)] // setAllowedContentTypes needs UTType; extensions are simpler here
    panel.setAllowedFileTypes(Some(&exts));
    panel.center();

    if panel.runModal() == NSModalResponseOK {
        if let Some(url) = panel.URLs().firstObject() {
            if let Some(path) = url.path() {
                set_model(app, &path.to_string());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Standalone NES-style window mirroring the tray menu
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct MenuChoice {
    id: String,
    label: String,
    active: bool,
    note: String,
}

#[derive(serde::Serialize)]
struct MenuState {
    has_engine: bool,
    status: String,
    icon_dark: bool,
    models: Vec<MenuChoice>,
    custom_model: Option<String>,
    hotkeys: Vec<MenuChoice>,
    custom_hotkey: Option<String>,
    recent: Vec<String>,
}

/// Snapshot of everything the window renders.
#[tauri::command]
fn menu_state(app: AppHandle) -> MenuState {
    let state = app.state::<AppState>();
    let current_model = state.current_model.lock().unwrap().clone();
    let current_shortcut = state.current_shortcut.lock().unwrap().clone();
    let hotkey_mode = state.cfg.lock().unwrap().hotkey_mode.clone();
    let has_engine = state.engine.lock().unwrap().is_some();
    let recent = history::recent(HISTORY_MENU_ITEMS);

    let models = MODEL_CHOICES
        .iter()
        .map(|(name, label)| MenuChoice {
            id: (*name).to_string(),
            label: (*label).to_string(),
            active: *name == current_model,
            note: if model::model_path(name).exists() {
                String::new()
            } else {
                "needs download".to_string()
            },
        })
        .collect();
    let model_listed = MODEL_CHOICES.iter().any(|(n, _)| *n == current_model);
    let custom_model = if model_listed || current_model.is_empty() {
        None
    } else if current_model.starts_with('/') {
        std::path::Path::new(&current_model)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
    } else {
        Some(current_model.clone())
    };

    let hotkeys = HOTKEY_CHOICES
        .iter()
        .map(|(accel, label)| MenuChoice {
            id: (*accel).to_string(),
            label: (*label).to_string(),
            active: *accel == current_shortcut,
            note: String::new(),
        })
        .collect();
    let hotkey_listed = HOTKEY_CHOICES.iter().any(|(a, _)| *a == current_shortcut);
    let custom_hotkey = if hotkey_listed {
        None
    } else {
        Some(current_shortcut.clone())
    };

    MenuState {
        has_engine,
        status: if has_engine {
            format!("READY · {current_shortcut} · {hotkey_mode}")
        } else {
            "NO MODEL — PICK ONE BELOW".to_string()
        },
        icon_dark: state.icon_dark.load(Ordering::SeqCst),
        models,
        custom_model,
        hotkeys,
        custom_hotkey,
        recent,
    }
}

#[tauri::command]
fn menu_choose_model(app: AppHandle, name: String) {
    set_model(&app, &name);
}

#[tauri::command]
fn menu_model_from_file(app: AppHandle) {
    pick_model_file(&app);
}

#[tauri::command]
fn menu_custom_model(app: AppHandle) {
    open_model_window(&app);
}

#[tauri::command]
fn menu_choose_hotkey(app: AppHandle, accel: String) -> Result<(), String> {
    set_hotkey(&app, &accel)
}

#[tauri::command]
fn menu_record_hotkey(app: AppHandle) {
    open_hotkey_window(&app);
}

#[tauri::command]
fn menu_toggle_theme(app: AppHandle) {
    toggle_icon_theme(&app);
}

#[tauri::command]
fn menu_reload_config(app: AppHandle) {
    reload_config(&app);
}

#[tauri::command]
fn menu_open_setup(app: AppHandle) {
    open_setup_window(&app);
}

#[tauri::command]
fn menu_type_recent(index: usize) {
    let recent = history::recent(HISTORY_MENU_ITEMS);
    if let Some(text) = recent.get(index).cloned() {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            if let Err(e) = inject::type_text(&text) {
                log::error!("{e}");
            }
        });
    }
}

#[tauri::command]
fn menu_quit(app: AppHandle) {
    app.exit(0);
}

/// Open (or focus) the standalone NES-style main window. Stays a menu-bar
/// (Accessory) app — no Dock icon — and surfaces the window explicitly with
/// show()/set_focus() after centering it.
fn open_main_window(app: &AppHandle) {
    activate_app();
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    // Native window with an overlay (transparent) title bar: keeps the dark
    // themed look while giving real traffic-light controls (close / minimize /
    // fullscreen) and native title-bar dragging.
    let result =
        tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("menu.html".into()))
            .title("Gretchen Flow")
            .inner_size(440.0, 620.0)
            .min_inner_size(360.0, 480.0)
            .resizable(true)
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true)
            .focused(true)
            .build();
    match result {
        Ok(window) => {
            let _ = window.center();
            let _ = window.show();
            let _ = window.set_focus();
        }
        Err(e) => log::error!("couldn't open main window: {e}"),
    }
}

#[tauri::command]
fn menu_close(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.close();
    }
}

/// Show the first-run / setup guide on the main thread.
fn show_setup(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || open_setup_window(&handle));
}

/// Open (or focus) the setup window explaining how to download a model and
/// grant permissions.
fn open_setup_window(app: &AppHandle) {
    activate_app();
    if let Some(window) = app.get_webview_window("setup") {
        let _ = window.set_focus();
        return;
    }
    let result =
        tauri::WebviewWindowBuilder::new(app, "setup", tauri::WebviewUrl::App("setup.html".into()))
            .title("Welcome to Gretchen Flow")
            .inner_size(520.0, 580.0)
            .resizable(false)
            .center()
            .build();
    if let Err(e) = result {
        log::error!("couldn't open setup window: {e}");
    }
}

#[tauri::command]
fn download_recommended_model(app: AppHandle) {
    set_model(&app, RECOMMENDED_MODEL);
    if let Some(window) = app.get_webview_window("setup") {
        let _ = window.close();
    }
}

#[tauri::command]
fn close_setup(app: AppHandle) {
    if let Some(window) = app.get_webview_window("setup") {
        let _ = window.close();
    }
}

/// Open (or focus) the custom model input window.
fn open_model_window(app: &AppHandle) {
    activate_app();
    if let Some(window) = app.get_webview_window("model") {
        let _ = window.set_focus();
        return;
    }
    let result =
        tauri::WebviewWindowBuilder::new(app, "model", tauri::WebviewUrl::App("model.html".into()))
            .title("Custom Model — Gretchen Flow")
            .inner_size(420.0, 250.0)
            .resizable(false)
            .always_on_top(true)
            .build();
    if let Err(e) = result {
        log::error!("couldn't open model window: {e}");
    }
}

#[tauri::command]
async fn apply_custom_model(app: AppHandle, name: String) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() || name.contains(['/', ' ', '\\']) {
        return Err("Enter a plain model name like \"medium.en\"".into());
    }
    // Validate the model exists in the collection before kicking off a
    // potentially large download.
    if !model::model_path(&name).exists() {
        let url =
            format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{name}.bin");
        ureq::head(&url)
            .call()
            .map_err(|_| format!("No model named \"{name}\" in the whisper.cpp collection"))?;
    }
    set_model(&app, &name);
    if let Some(window) = app.get_webview_window("model") {
        let _ = window.close();
    }
    Ok(())
}

#[tauri::command]
fn cancel_custom_model(app: AppHandle) {
    if let Some(window) = app.get_webview_window("model") {
        let _ = window.close();
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
    let hold_mode = state.cfg.lock().unwrap().hotkey_mode == "hold";
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

/// Handles to the menu items that change at runtime. The menu is built once
/// and mutated in place — replacing a tray menu (or its submenus) while macOS
/// has shown it once makes the dropdown glitch on the next hover.
struct MenuHandles {
    menu: Menu<tauri::Wry>,
    status: MenuItem<tauri::Wry>,
    hist_header: MenuItem<tauri::Wry>,
    hist: Vec<MenuItem<tauri::Wry>>,
    model_items: Vec<CheckMenuItem<tauri::Wry>>,
    model_custom: CheckMenuItem<tauri::Wry>,
    hotkey_items: Vec<CheckMenuItem<tauri::Wry>>,
    hotkey_custom: CheckMenuItem<tauri::Wry>,
    theme: MenuItem<tauri::Wry>,
}

thread_local! {
    /// Menu handles live on the main thread only (AppKit menu items are not
    /// thread-safe); all access goes through run_on_main_thread.
    static MENU_HANDLES: std::cell::RefCell<Option<MenuHandles>> =
        const { std::cell::RefCell::new(None) };
}

/// Build the full tray menu once, with fixed slots for everything dynamic.
fn build_menu(app: &AppHandle) -> tauri::Result<MenuHandles> {
    let menu = Menu::new(app)?;

    let status = MenuItem::with_id(app, "status", "Gretchen Flow", false, None::<&str>)?;
    menu.append(&status)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    let hist_header =
        MenuItem::with_id(app, "hist-header", "No dictations yet", false, None::<&str>)?;
    menu.append(&hist_header)?;
    let mut hist = Vec::new();
    for i in 0..HISTORY_MENU_ITEMS {
        let item = MenuItem::with_id(app, format!("hist-{i}"), "—", false, None::<&str>)?;
        menu.append(&item)?;
        hist.push(item);
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    let model_menu = Submenu::with_id(app, "model-menu", "Model", true)?;
    let mut model_items = Vec::new();
    for (name, label) in MODEL_CHOICES {
        let item = CheckMenuItem::with_id(
            app,
            format!("model-{name}"),
            *label,
            true,
            false,
            None::<&str>,
        )?;
        model_menu.append(&item)?;
        model_items.push(item);
    }
    let model_custom =
        CheckMenuItem::with_id(app, "model-custom-slot", "—", false, false, None::<&str>)?;
    model_menu.append(&model_custom)?;
    model_menu.append(&PredefinedMenuItem::separator(app)?)?;
    model_menu.append(&MenuItem::with_id(
        app,
        "custom-model",
        "Custom Model…",
        true,
        None::<&str>,
    )?)?;
    model_menu.append(&MenuItem::with_id(
        app,
        "model-file",
        "Model from File…",
        true,
        None::<&str>,
    )?)?;
    menu.append(&model_menu)?;

    let hotkey_menu = Submenu::with_id(app, "hotkey-menu", "Hotkey", true)?;
    let mut hotkey_items = Vec::new();
    for (accel, label) in HOTKEY_CHOICES {
        let item = CheckMenuItem::with_id(
            app,
            format!("hotkey-{accel}"),
            *label,
            true,
            false,
            None::<&str>,
        )?;
        hotkey_menu.append(&item)?;
        hotkey_items.push(item);
    }
    let hotkey_custom =
        CheckMenuItem::with_id(app, "hotkey-custom-slot", "—", false, false, None::<&str>)?;
    hotkey_menu.append(&hotkey_custom)?;
    hotkey_menu.append(&PredefinedMenuItem::separator(app)?)?;
    hotkey_menu.append(&MenuItem::with_id(
        app,
        "record-hotkey",
        "Set Custom Hotkey…",
        true,
        None::<&str>,
    )?)?;
    menu.append(&hotkey_menu)?;

    let theme = MenuItem::with_id(app, "theme", "Icon", true, None::<&str>)?;
    menu.append(&theme)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "open-window",
        "Open Gretchen Flow…",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "setup",
        "Getting Started…",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "reload-config",
        "Reload Config",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        "Quit Gretchen Flow",
        true,
        None::<&str>,
    )?)?;

    Ok(MenuHandles {
        menu,
        status,
        hist_header,
        hist,
        model_items,
        model_custom,
        hotkey_items,
        hotkey_custom,
        theme,
    })
}

/// Update the tray menu in place on the main thread.
fn refresh_menu(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || refresh_menu_on_main(&handle));
}

fn refresh_menu_on_main(app: &AppHandle) {
    let state = app.state::<AppState>();
    let recent = history::recent(HISTORY_MENU_ITEMS);
    let current_shortcut = state.current_shortcut.lock().unwrap().clone();
    let current_model = state.current_model.lock().unwrap().clone();
    let hotkey_mode = state.cfg.lock().unwrap().hotkey_mode.clone();
    let has_engine = state.engine.lock().unwrap().is_some();
    let dark = state.icon_dark.load(Ordering::SeqCst);

    MENU_HANDLES.with(|handles| {
        let handles = handles.borrow();
        let Some(h) = handles.as_ref() else { return };

        let _ = h.status.set_text(if has_engine {
            format!("Gretchen Flow — {current_shortcut} ({hotkey_mode})")
        } else {
            "No model — open Model ▸ to download one".to_string()
        });

        let _ = h.hist_header.set_text(if recent.is_empty() {
            "No dictations yet"
        } else {
            "Recent — click to type again"
        });
        for (i, item) in h.hist.iter().enumerate() {
            match recent.get(i) {
                Some(text) => {
                    let label: String = if text.chars().count() > 45 {
                        format!("{}…", text.chars().take(45).collect::<String>())
                    } else {
                        text.clone()
                    };
                    let _ = item.set_text(label);
                    let _ = item.set_enabled(true);
                }
                None => {
                    let _ = item.set_text("—");
                    let _ = item.set_enabled(false);
                }
            }
        }

        let mut model_listed = false;
        for ((name, label), item) in MODEL_CHOICES.iter().zip(&h.model_items) {
            let checked = *name == current_model;
            model_listed |= checked;
            let text = if model::model_path(name).exists() {
                (*label).to_string()
            } else {
                format!("{label}  (needs download)")
            };
            let _ = item.set_text(text);
            let _ = item.set_checked(checked);
        }
        if model_listed || current_model.is_empty() {
            // No custom model in use (or none selected at all).
            let _ = h.model_custom.set_text("—");
            let _ = h.model_custom.set_enabled(false);
            let _ = h.model_custom.set_checked(false);
        } else {
            // File-path models display as their file name.
            let label = if current_model.starts_with('/') {
                std::path::Path::new(&current_model)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| current_model.clone())
            } else {
                current_model.clone()
            };
            let _ = h.model_custom.set_text(label);
            let _ = h.model_custom.set_enabled(true);
            let _ = h.model_custom.set_checked(true);
        }

        let mut hotkey_listed = false;
        for ((accel, _), item) in HOTKEY_CHOICES.iter().zip(&h.hotkey_items) {
            let checked = *accel == current_shortcut;
            hotkey_listed |= checked;
            let _ = item.set_checked(checked);
        }
        if hotkey_listed {
            let _ = h.hotkey_custom.set_text("—");
            let _ = h.hotkey_custom.set_enabled(false);
            let _ = h.hotkey_custom.set_checked(false);
        } else {
            let _ = h.hotkey_custom.set_text(current_shortcut.clone());
            let _ = h.hotkey_custom.set_enabled(true);
            let _ = h.hotkey_custom.set_checked(true);
        }

        let _ = h.theme.set_text(if dark {
            "Icon: Dark — switch to Light"
        } else {
            "Icon: Light — switch to Dark"
        });
    });

    *state.history_items.lock().unwrap() = recent;

    // Keep the standalone window in sync if it's open.
    use tauri::Emitter;
    let _ = app.emit("menu-updated", ());
}

fn on_menu_event(app: &AppHandle, id: &str) {
    log::info!("menu event: {id}");
    if id == "quit" {
        app.exit(0);
        return;
    }
    if id == "model-file" {
        pick_model_file(app);
        return;
    }
    if id == "reload-config" {
        reload_config(app);
        return;
    }
    if id == "setup" {
        show_setup(app);
        return;
    }
    if id == "open-window" {
        open_main_window(app);
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
    if id == "custom-model" {
        open_model_window(app);
        return;
    }
    if id == "model-custom-slot" || id == "hotkey-custom-slot" {
        return; // already-active custom entries — nothing to do
    }
    if let Some(accel) = id.strip_prefix("hotkey-") {
        if accel != "menu" {
            if let Err(e) = set_hotkey(app, accel) {
                log::error!("{e}");
            }
        }
        return;
    }
    if let Some(name) = id.strip_prefix("model-") {
        if name != "menu" {
            set_model(app, name);
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

/// Load the configured model on startup IF it's already on disk. The app
/// ships with no model and never auto-downloads — a missing model opens the
/// setup guide instead.
fn load_engine_async(app: AppHandle) {
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let cfg = state.cfg.lock().unwrap().clone();

        let available = if cfg.model.is_empty() {
            false
        } else if cfg.model.starts_with('/') {
            std::path::Path::new(&cfg.model).exists()
        } else {
            model::model_path(&cfg.model).exists()
        };
        if !available {
            log::info!("no model downloaded yet; opening setup");
            set_tray_state(&app, TrayState::NeedsModel);
            show_setup(&app);
            return;
        }

        set_tray_state(&app, TrayState::Downloading);
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

/// Force-reload the transcription engine from the current config, keeping the
/// existing engine until the new one is ready. Picks up model and
/// transcription-setting changes (language, vocabulary, punctuation, etc.).
fn reload_engine(app: AppHandle, model: String) {
    std::thread::spawn(move || {
        set_tray_state(&app, TrayState::Downloading);
        let cfg = config::Config::load();
        let loaded = model::ensure_model(&model)
            .and_then(|path| transcribe::Engine::load(&path.to_string_lossy(), &cfg));
        let state = app.state::<AppState>();
        match loaded {
            Ok(engine) => {
                *state.engine.lock().unwrap() = Some(engine);
                log::info!("engine reloaded (model: {model})");
            }
            Err(e) => log::error!("engine reload failed, keeping current engine: {e}"),
        }
        set_tray_state(&app, TrayState::Idle);
    });
}

/// Menu action: re-read config.json and apply everything live — hotkey,
/// icon theme, model, and transcription settings — without restarting.
fn reload_config(app: &AppHandle) {
    let fresh = config::Config::load();
    let state = app.state::<AppState>();

    // Re-register the hotkey if it changed.
    let current_shortcut = state.current_shortcut.lock().unwrap().clone();
    if fresh.shortcut != current_shortcut {
        if let Err(e) = set_hotkey(app, &fresh.shortcut) {
            log::error!("reload config: {e}");
        }
    }

    // Apply the icon theme if it changed.
    let want_dark = fresh.icon_theme != "light";
    if want_dark != state.icon_dark.load(Ordering::SeqCst) {
        state.icon_dark.store(want_dark, Ordering::SeqCst);
        if !state.recording.load(Ordering::SeqCst) {
            set_tray_state(app, TrayState::Idle);
        }
    }

    // Swap in the new config (covers hotkey_mode and any future fields).
    let model = fresh.model.clone();
    *state.cfg.lock().unwrap() = fresh;
    *state.current_model.lock().unwrap() = model.clone();

    refresh_menu(app);
    log::info!("config reloaded");

    // Always reload the engine so language/vocabulary/punctuation edits apply
    // even when the model name is unchanged.
    reload_engine(app.clone(), model);
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

    // "Fn" is handled by the low-level listener, not a registered shortcut.
    let shortcut: Option<Shortcut> = if cfg.shortcut == FN_HOTKEY {
        None
    } else {
        Some(
            cfg.shortcut
                .parse()
                .expect("invalid shortcut in config.json"),
        )
    };

    // Surface panics in the log — an unwind across a system callback aborts
    // the process with no crash report otherwise.
    std::panic::set_hook(Box::new(|info| {
        log::error!("PANIC: {info}");
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            apply_custom_hotkey,
            cancel_custom_hotkey,
            apply_custom_model,
            cancel_custom_model,
            download_recommended_model,
            close_setup,
            menu_state,
            menu_choose_model,
            menu_model_from_file,
            menu_custom_model,
            menu_choose_hotkey,
            menu_record_hotkey,
            menu_toggle_theme,
            menu_reload_config,
            menu_open_setup,
            menu_type_recent,
            menu_quit,
            menu_close
        ])
        .manage(AppState {
            recorder: audio::Recorder::spawn(),
            engine: Arc::new(Mutex::new(None)),
            recording: AtomicBool::new(false),
            icon_dark: AtomicBool::new(cfg.icon_theme != "light"),
            current_shortcut: Mutex::new(cfg.shortcut.clone()),
            fn_hotkey: AtomicBool::new(cfg.shortcut == FN_HOTKEY),
            current_model: Mutex::new(cfg.model.clone()),
            cfg: Mutex::new(cfg),
            history_items: Mutex::new(Vec::new()),
        })
        .setup(move |app| {
            // Regular app: appears in the Dock (clicking it opens the window),
            // and also keeps a menu-bar tray icon for recording status.
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Regular);
                set_dock_icon();
            }

            let dark = app.state::<AppState>().icon_dark.load(Ordering::SeqCst);
            let initial_icon = if dark {
                ICON_IDLE_DARK
            } else {
                ICON_IDLE_LIGHT
            };
            let handles = build_menu(app.handle())?;
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(Image::from_bytes(initial_icon)?)
                .icon_as_template(!dark)
                .menu(&handles.menu)
                .on_menu_event(|app, event| on_menu_event(app, event.id().as_ref()))
                .build(app)?;
            MENU_HANDLES.with(|cell| *cell.borrow_mut() = Some(handles));
            refresh_menu(app.handle());

            if let Some(shortcut) = shortcut {
                app.global_shortcut()
                    .on_shortcut(shortcut, |app, _sc, event| on_shortcut(app, event.state()))?;
            }
            spawn_fn_listener(app.handle().clone());

            load_engine_async(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Gretchen Flow")
        .run(|app, event| match event {
            // Keep running with zero windows; only exit via the Quit menu item.
            tauri::RunEvent::ExitRequested { code, api, .. } => {
                log::info!("exit requested (code: {code:?})");
                if code.is_none() {
                    api.prevent_exit();
                }
            }
            // Clicking the app icon (Finder/Dock/Launchpad) while it's already
            // running re-opens the standalone window.
            tauri::RunEvent::Reopen { .. } => open_main_window(app),
            tauri::RunEvent::Exit => log::info!("exiting"),
            _ => {}
        });
}
