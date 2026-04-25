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
    let old_hotkey = state.settings.lock().hotkey.clone();
    let new_hotkey = settings.hotkey.clone();

    settings.save(&state.app_dir)?;
    *state.settings.lock() = settings;

    if old_hotkey != new_hotkey {
        if let Err(e) = rebind_hotkey(&app, &old_hotkey, &new_hotkey) {
            // Roll back to the old hotkey in memory + on disk so we don't
            // strand the user with no working shortcut.
            warn!("Hotkey rebind failed ({}). Reverting.", e);
            let mut s = state.settings.lock();
            s.hotkey = old_hotkey.clone();
            let _ = s.save(&state.app_dir);
            return Err(format!("Could not bind '{}': {}. Reverted.", new_hotkey, e));
        }
        info!("Hotkey rebound: {} → {}", old_hotkey, new_hotkey);
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
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

async fn do_toggle_recording(
    app: &AppHandle,
    state: &AppState,
) -> Result<String, String> {
    match state.recorder.get_state() {
        RecordingState::Ready => {
            let mic = state.settings.lock().microphone.clone();
            state.recorder.start_recording(app, &mic)?;
            Ok("recording".to_string())
        }
        RecordingState::Recording => {
            let settings = state.settings.lock().clone();
            state.recorder.stop_and_transcribe(app, &settings).await
        }
        RecordingState::Transcribing => {
            Err("Currently transcribing, please wait".to_string())
        }
    }
}

fn register_recording_hotkey(handle: &AppHandle, accelerator: &str) -> Result<(), String> {
    let captured = handle.clone();
    handle
        .global_shortcut()
        .on_shortcut(accelerator, move |_app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            let handle = captured.clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                match do_toggle_recording(&handle, state.inner()).await {
                    Ok(r) => info!("Recording transition: {}", r),
                    Err(e) => {
                        // Benign — user pressed too fast while transcribing.
                        if e.contains("transcribing") {
                            return;
                        }
                        error!("Toggle failed: {}", e);
                        let _ = handle.emit("recording-error", &e);
                    }
                }
            });
        })
        .map_err(|e| e.to_string())
}

fn rebind_hotkey(handle: &AppHandle, old: &str, new: &str) -> Result<(), String> {
    let shortcut = handle.global_shortcut();
    if !old.is_empty() {
        if let Err(e) = shortcut.unregister(old) {
            warn!("Failed to unregister '{}': {}", old, e);
        }
    }
    register_recording_hotkey(handle, new)
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

            if api_key_missing {
                open_settings(handle.clone());
            }

            info!("Registering hotkey: {}", initial_hotkey);
            if let Err(e) = register_recording_hotkey(&handle, &initial_hotkey) {
                error!("Failed to register hotkey '{}': {}", initial_hotkey, e);
                let _ = handle.emit(
                    "recording-error",
                    format!("Could not bind hotkey '{}': {}", initial_hotkey, e),
                );
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

