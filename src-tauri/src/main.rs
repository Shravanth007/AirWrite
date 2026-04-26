#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use airwrite_lib::audio;
use airwrite_lib::recorder::{Recorder, RecordingState};
use airwrite_lib::settings::Settings;

use log::{error, info, warn};
use parking_lot::Mutex;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

struct AppState {
    recorder: Recorder,
    settings: Mutex<Settings>,
    app_dir: PathBuf,
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
    let old = state.settings.lock().clone();

    settings.save(&state.app_dir)?;
    *state.settings.lock() = settings.clone();

    // Recording hotkey
    if old.hotkey != settings.hotkey {
        if let Err(e) = rebind_recording_hotkey(&app, &old.hotkey, &settings.hotkey) {
            warn!("Recording hotkey rebind failed ({}). Reverting.", e);
            let mut s = state.settings.lock();
            s.hotkey = old.hotkey.clone();
            let _ = s.save(&state.app_dir);
            return Err(format!(
                "Could not bind recording hotkey '{}': {}. Reverted.",
                settings.hotkey, e
            ));
        }
        info!("Recording hotkey rebound: {} → {}", old.hotkey, settings.hotkey);
    }

    // Settings (open-panel) hotkey
    if old.settings_hotkey != settings.settings_hotkey {
        if let Err(e) =
            rebind_settings_hotkey(&app, &old.settings_hotkey, &settings.settings_hotkey)
        {
            warn!("Settings hotkey rebind failed ({}). Reverting.", e);
            let mut s = state.settings.lock();
            s.settings_hotkey = old.settings_hotkey.clone();
            let _ = s.save(&state.app_dir);
            return Err(format!(
                "Could not bind settings hotkey '{}': {}. Reverted.",
                settings.settings_hotkey, e
            ));
        }
        info!(
            "Settings hotkey rebound: {} → {}",
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
                let mic = state.settings.lock().microphone.clone();
                state.recorder.start_recording(app, &mic)?;
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
                    let mic = state.settings.lock().microphone.clone();
                    state.recorder.start_recording(app, &mic)?;
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
    let shortcut = handle.global_shortcut();
    if !old.is_empty() {
        if let Err(e) = shortcut.unregister(old) {
            warn!("Failed to unregister '{}': {}", old, e);
        }
    }
    register_recording_hotkey(handle, new)
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
/// focus, visible & focused → hide. The tray menu's "Settings" entry
/// deliberately uses `open_settings` (always-show) instead — clicking a menu
/// item is unambiguous intent to see the window.
fn toggle_settings_window(handle: &AppHandle) {
    let Some(w) = handle.get_webview_window("settings") else {
        warn!("toggle_settings_window: settings window not found");
        return;
    };
    let visible = w.is_visible().unwrap_or(false);
    let focused = w.is_focused().unwrap_or(false);
    if visible && focused {
        let _ = w.hide();
    } else {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn rebind_settings_hotkey(handle: &AppHandle, old: &str, new: &str) -> Result<(), String> {
    let shortcut = handle.global_shortcut();
    if !old.is_empty() {
        if let Err(e) = shortcut.unregister(old) {
            warn!("Failed to unregister settings hotkey '{}': {}", old, e);
        }
    }
    register_settings_hotkey(handle, new)
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

    TrayIconBuilder::new()
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
    let tray_tooltip = format!("AirWrite — {} to dictate", initial_hotkey);

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            recorder: Recorder::new(),
            settings: Mutex::new(settings),
            app_dir: dir,
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

            if let Err(e) = build_tray(&handle, &tray_tooltip) {
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
                    format!("Could not bind hotkey '{}': {}", initial_hotkey, e),
                );
            }

            info!("Registering settings hotkey: {}", initial_settings_hotkey);
            if let Err(e) = register_settings_hotkey(&handle, &initial_settings_hotkey) {
                // Non-fatal: user can still open Settings from the tray.
                warn!(
                    "Failed to register settings hotkey '{}': {}",
                    initial_settings_hotkey, e
                );
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

