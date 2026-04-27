#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use airwrite_lib::audio;
use airwrite_lib::recorder::{Recorder, RecordingState};
use airwrite_lib::settings::Settings;

use log::{error, info, warn};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// Window of suppression for the settings toggle hotkey. Windows reports a
/// brief is_focused=false right after show() before paint settles, and humans
/// double-tap accelerators all the time. Anything inside this window after
/// the previous toggle is treated as a duplicate and ignored.
const SETTINGS_TOGGLE_DEBOUNCE: Duration = Duration::from_millis(250);

/// Stable id for the system tray icon. We look the icon up by id from
/// `save_settings` so a hotkey change can refresh the tooltip in place.
const TRAY_ID: &str = "airwrite-tray";

fn tray_tooltip(hotkey: &str) -> String {
    format!("AirWrite — {} to dictate", hotkey)
}

struct AppState {
    recorder: Recorder,
    settings: Mutex<Settings>,
    app_dir: PathBuf,
    /// Timestamp of the last settings-window show/hide. Used by
    /// `toggle_settings_window` to debounce rapid presses.
    last_settings_toggle: Mutex<Option<Instant>>,
}

fn app_dir() -> PathBuf {
    // LocalAppData on Windows — secrets-adjacent state should NOT roam.
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.airwrite.app")
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().clone()
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<AppState>,
    settings: Settings,
) -> Result<(), String> {
    // Validate before any side effects: reject hotkey conflicts so we never
    // try to register the same accelerator twice.
    let recording = settings.hotkey.trim();
    let panel = settings.settings_hotkey.trim();
    if !recording.is_empty() && !panel.is_empty() && recording == panel {
        return Err(
            "Recording and Settings hotkeys can't be the same combination."
                .to_string(),
        );
    }

    // Snapshot the previous state so we can roll back hotkey rebinds if the
    // second one fails (the first is already applied at the OS level).
    let old = state.settings.lock().clone();
    let recording_changed = old.hotkey != settings.hotkey;
    let panel_changed = old.settings_hotkey != settings.settings_hotkey;

    // Apply hotkey changes FIRST. The rebind helpers register-new-then-
    // unregister-old, so a failure on either leaves the previous hotkey
    // intact at the OS level. If both rebinds need to run and the second
    // fails, we explicitly roll the first one back (see below). Disk write
    // only happens once every requested rebind has succeeded.
    if recording_changed {
        rebind_recording_hotkey(&app, &old.hotkey, &settings.hotkey)
            .map_err(|e| format!("Could not bind recording hotkey '{}': {}", settings.hotkey, e))?;
    }
    if panel_changed {
        if let Err(e) =
            rebind_settings_hotkey(&app, &old.settings_hotkey, &settings.settings_hotkey)
        {
            // Settings hotkey failed — undo the recording rebind so the user
            // isn't left with a half-applied combo.
            if recording_changed {
                if let Err(re) = rebind_recording_hotkey(&app, &settings.hotkey, &old.hotkey) {
                    error!(
                        "Recovery rebind of recording hotkey '{}' failed: {}",
                        old.hotkey, re
                    );
                }
            }
            return Err(format!(
                "Could not bind settings hotkey '{}': {}",
                settings.settings_hotkey, e
            ));
        }
    }

    // Both rebinds succeeded (or there were none). Now commit to disk and
    // memory. Any failure here is best-effort — the OS already has the new
    // hotkeys, and the next restart will reload from disk.
    settings.save(&state.app_dir)?;
    *state.settings.lock() = settings.clone();

    if recording_changed {
        info!("Recording hotkey: {} → {}", old.hotkey, settings.hotkey);
        // Tray tooltip mentions the recording hotkey — keep it in sync so
        // the user doesn't see the old combo there after changing it.
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            if let Err(e) = tray.set_tooltip(Some(tray_tooltip(&settings.hotkey))) {
                warn!("Could not update tray tooltip: {}", e);
            }
        }
    }
    if panel_changed {
        info!(
            "Settings hotkey: {} → {}",
            old.settings_hotkey, settings.settings_hotkey
        );
    }
    Ok(())
}

#[tauri::command]
fn list_microphones() -> Vec<audio::MicDevice> {
    audio::list_microphones()
}

#[tauri::command]
async fn test_microphone(
    state: State<'_, AppState>,
    mic: Option<String>,
) -> Result<audio::MicTestResult, String> {
    let name = mic.unwrap_or_else(|| state.settings.lock().microphone.clone());
    // Off the main thread — CPAL start/stop on its own task thread is fine here
    // because the test is self-contained (no overlap with the main recorder).
    tauri::async_runtime::spawn_blocking(move || audio::test_microphone(&name, 1500))
        .await
        .map_err(|e| format!("Test thread panicked: {}", e))?
}

#[tauri::command]
fn open_mic_privacy_settings() -> Result<(), String> {
    // ms-settings: URI handlers open the right panel directly.
    std::process::Command::new("cmd")
        .args(["/C", "start", "", "ms-settings:privacy-microphone"])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to open Windows settings: {}", e))
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

/// Dispatch one hotkey event into the recorder. Branches on the user's
/// `recording_mode` setting at the moment the key is hit, so toggling the
/// mode in Settings takes effect on the next press without a restart.
async fn handle_hotkey_event(
    app: &AppHandle,
    state: &AppState,
    pressed: bool,
) -> Result<&'static str, String> {
    let mode = state.settings.lock().recording_mode.clone();

    match mode.as_str() {
        // Press = start, release = stop & transcribe.
        "push_to_talk" => match (pressed, state.recorder.get_state()) {
            (true, RecordingState::Ready) => {
                let settings = state.settings.lock().clone();
                state.recorder.start_recording(app, &settings)?;
                Ok("ptt: started")
            }
            (false, RecordingState::Recording) => {
                let settings = state.settings.lock().clone();
                state
                    .recorder
                    .stop_and_transcribe(app, &settings)
                    .await
                    .map(|_| "ptt: stopped & transcribed")
            }
            // Press while we're still transcribing the previous burst, or
            // release without a matching press — both are benign no-ops.
            _ => Ok("ptt: noop"),
        },

        // Toggle (default): act only on press.
        _ => {
            if !pressed {
                return Ok("toggle: ignored release");
            }
            match state.recorder.get_state() {
                RecordingState::Ready => {
                    let settings = state.settings.lock().clone();
                    state.recorder.start_recording(app, &settings)?;
                    Ok("toggle: started")
                }
                RecordingState::Recording => {
                    let settings = state.settings.lock().clone();
                    state
                        .recorder
                        .stop_and_transcribe(app, &settings)
                        .await
                        .map(|_| "toggle: stopped & transcribed")
                }
                RecordingState::Transcribing => {
                    Err("Currently transcribing, please wait".to_string())
                }
            }
        }
    }
}

fn register_recording_hotkey(handle: &AppHandle, accelerator: &str) -> Result<(), String> {
    let captured = handle.clone();
    handle
        .global_shortcut()
        .on_shortcut(accelerator, move |_app, _shortcut, event| {
            let pressed = event.state == ShortcutState::Pressed;
            let handle = captured.clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                match handle_hotkey_event(&handle, state.inner(), pressed).await {
                    Ok(r) => info!("Hotkey: {}", r),
                    Err(e) => {
                        // Benign — user mashed the key during transcribing.
                        if e.contains("transcribing") {
                            return;
                        }
                        error!("Hotkey failed: {}", e);
                        let _ = handle.emit("recording-error", &e);
                    }
                }
            });
        })
        .map_err(|e| e.to_string())
}

fn rebind_recording_hotkey(handle: &AppHandle, old: &str, new: &str) -> Result<(), String> {
    // Register the new accelerator FIRST. If that fails (combo already in
    // use, malformed, etc.) we surface the error with the user's previous
    // hotkey still bound — they don't get stranded with no working key.
    // Only after `new` is live do we drop `old`.
    register_recording_hotkey(handle, new)?;
    if !old.is_empty() && old != new {
        if let Err(e) = handle.global_shortcut().unregister(old) {
            warn!("Failed to unregister old recording hotkey '{}': {}", old, e);
        }
    }
    Ok(())
}

fn register_settings_hotkey(handle: &AppHandle, accelerator: &str) -> Result<(), String> {
    if accelerator.is_empty() {
        return Ok(());
    }
    let captured = handle.clone();
    handle
        .global_shortcut()
        .on_shortcut(accelerator, move |_app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            toggle_settings_window(&captured);
        })
        .map_err(|e| e.to_string())
}

/// Hotkey-driven toggle: hidden → show & focus, visible-but-unfocused →
/// focus, visible & focused → hide. Debounced so rapid double-presses (and
/// the focus race that Windows triggers right after `show()`) don't ping-pong
/// the window state. The tray menu's "Settings" entry deliberately uses
/// `open_settings` (always-show) instead — clicking a menu item is
/// unambiguous intent to see the window.
fn toggle_settings_window(handle: &AppHandle) {
    let Some(w) = handle.get_webview_window("settings") else {
        warn!("toggle_settings_window: settings window not found");
        return;
    };
    let state = handle.state::<AppState>();

    // Suppress if we acted on this hotkey within the debounce window.
    {
        let last = state.last_settings_toggle.lock();
        if let Some(t) = *last {
            if t.elapsed() < SETTINGS_TOGGLE_DEBOUNCE {
                return;
            }
        }
    }

    let visible = w.is_visible().unwrap_or(false);
    let focused = w.is_focused().unwrap_or(false);
    if visible && focused {
        let _ = w.hide();
    } else {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
    *state.last_settings_toggle.lock() = Some(Instant::now());
}

fn rebind_settings_hotkey(handle: &AppHandle, old: &str, new: &str) -> Result<(), String> {
    // Same atomic-rebind pattern as `rebind_recording_hotkey`: bind the new
    // combo first; only release the old one if the new bind succeeded. A
    // failure leaves the old hotkey working.
    register_settings_hotkey(handle, new)?;
    if !old.is_empty() && old != new {
        if let Err(e) = handle.global_shortcut().unregister(old) {
            warn!("Failed to unregister old settings hotkey '{}': {}", old, e);
        }
    }
    Ok(())
}

fn overlay_position(app: &AppHandle) -> (f64, f64) {
    if let Ok(Some(m)) = app.primary_monitor() {
        let scale = m.scale_factor();
        let logical_w = m.size().width as f64 / scale;
        // Center the compact pill horizontally near the top, like a notch.
        ((logical_w / 2.0) - 160.0, 18.0)
    } else {
        (760.0, 18.0)
    }
}

fn build_overlay_window(app: &AppHandle) -> tauri::Result<()> {
    let (x, y) = overlay_position(app);
    let w = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("/".into()))
        .title("")
        .inner_size(320.0, 52.0)
        .position(x, y)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .shadow(false)
        .visible(true)
        .build()?;
    // Click-through: never steal focus or block clicks.
    let _ = w.set_ignore_cursor_events(true);
    Ok(())
}

fn build_tray(app: &AppHandle, tooltip: &str) -> tauri::Result<()> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::TrayIconBuilder;

    let settings_item = MenuItemBuilder::new("Settings").id("settings").build(app)?;
    let quit_item = MenuItemBuilder::new("Quit").id("quit").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&settings_item, &quit_item])
        .build()?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().cloned().ok_or_else(|| {
            tauri::Error::AssetNotFound("default_window_icon".to_string())
        })?)
        .tooltip(tooltip)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "settings" => open_settings(app.clone()),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn init_logging() {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("airwrite=info,airwrite_lib=info,warn"),
    )
    .try_init();
}

fn main() {
    init_logging();

    let dir = app_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        warn!("Could not create app dir {}: {}", dir.display(), e);
    }
    let settings = Settings::load(&dir);
    let initial_hotkey = settings.hotkey.clone();
    let initial_settings_hotkey = settings.settings_hotkey.clone();
    let api_key_missing = settings.groq_api_key.trim().is_empty();
    let initial_tray_tooltip = tray_tooltip(&initial_hotkey);

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            recorder: Recorder::new(),
            settings: Mutex::new(settings),
            app_dir: dir,
            last_settings_toggle: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            list_microphones,
            test_microphone,
            open_mic_privacy_settings,
            open_settings,
            quit,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            if let Err(e) = build_tray(&handle, &initial_tray_tooltip) {
                error!("Tray init failed: {}", e);
            }

            match build_overlay_window(&handle) {
                Ok(_) => info!("Overlay window created"),
                Err(e) => error!("Failed to create overlay: {}", e),
            }

            // Intercept the Settings window's close button: hide instead of
            // destroy, so the next "Open settings" can find and re-show it.
            if let Some(settings_win) = handle.get_webview_window("settings") {
                let win = settings_win.clone();
                settings_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                });
            } else {
                warn!("Settings window not found at setup time — close-to-hide not wired");
            }

            if api_key_missing {
                open_settings(handle.clone());
            }

            info!("Registering recording hotkey: {}", initial_hotkey);
            if let Err(e) = register_recording_hotkey(&handle, &initial_hotkey) {
                error!("Failed to register hotkey '{}': {}", initial_hotkey, e);
                let _ = handle.emit(
                    "recording-error",
                    format!(
                        "Recording hotkey {} couldn't be bound — another app may already use it. Pick a different combination in Settings → Hotkey.",
                        initial_hotkey
                    ),
                );
            }

            info!("Registering settings hotkey: {}", initial_settings_hotkey);
            if let Err(e) = register_settings_hotkey(&handle, &initial_settings_hotkey) {
                warn!(
                    "Failed to register settings hotkey '{}': {}",
                    initial_settings_hotkey, e
                );
                let _ = handle.emit(
                    "recording-error",
                    format!(
                        "Settings hotkey {} couldn't be bound — another app may already use it. Pick a different combination in Settings → Hotkey.",
                        initial_settings_hotkey
                    ),
                );
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

