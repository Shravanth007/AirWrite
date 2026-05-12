#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use airwrite_lib::audio;
use airwrite_lib::ducking;
use airwrite_lib::history::{History, HistoryEntry};
use airwrite_lib::paste::paste_text;
use airwrite_lib::recorder::{Recorder, RecordingState};
use airwrite_lib::settings::Settings;

use log::{error, info, warn};
use parking_lot::Mutex;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const SETTINGS_TOGGLE_DEBOUNCE: Duration = Duration::from_millis(250);

const TRAY_ID: &str = "airwrite-tray";

fn tray_tooltip(hotkey: &str) -> String {
    format!("AirWrite â€” {} to dictate", hotkey)
}

struct AppState {
    recorder: Recorder,
    settings: Mutex<Settings>,
    app_dir: PathBuf,
    last_settings_toggle: Mutex<Option<Instant>>,
    registered_hotkeys: Mutex<HashSet<String>>,
    history: Arc<Mutex<History>>,
}

fn app_dir() -> PathBuf {
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
    let recording = settings.hotkey.trim();
    let panel = settings.settings_hotkey.trim();
    let repaste = settings.repaste_hotkey.trim();
    if !recording.is_empty() && !panel.is_empty() && recording == panel {
        return Err(
            "Recording and Settings hotkeys can't be the same combination."
                .to_string(),
        );
    }
    if !recording.is_empty() && !repaste.is_empty() && recording == repaste {
        return Err(
            "Recording and Re-paste hotkeys can't be the same combination."
                .to_string(),
        );
    }
    if !panel.is_empty() && !repaste.is_empty() && panel == repaste {
        return Err(
            "Settings and Re-paste hotkeys can't be the same combination."
                .to_string(),
        );
    }

    let old = state.settings.lock().clone();
    let recording_changed = old.hotkey != settings.hotkey;
    let panel_changed = old.settings_hotkey != settings.settings_hotkey;
    let repaste_changed = old.repaste_hotkey != settings.repaste_hotkey;

    if recording_changed {
        rebind_recording_hotkey(&app, &old.hotkey, &settings.hotkey)
            .map_err(|e| format!("Could not bind recording hotkey '{}': {}", settings.hotkey, e))?;
    }
    if panel_changed {
        if let Err(e) =
            rebind_settings_hotkey(&app, &old.settings_hotkey, &settings.settings_hotkey)
        {
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
    if repaste_changed {
        if let Err(e) = rebind_repaste_hotkey(&app, &old.repaste_hotkey, &settings.repaste_hotkey)
        {
            if panel_changed {
                if let Err(re) = rebind_settings_hotkey(
                    &app,
                    &settings.settings_hotkey,
                    &old.settings_hotkey,
                ) {
                    error!(
                        "Recovery rebind of settings hotkey '{}' failed: {}",
                        old.settings_hotkey, re
                    );
                }
            }
            if recording_changed {
                if let Err(re) = rebind_recording_hotkey(&app, &settings.hotkey, &old.hotkey) {
                    error!(
                        "Recovery rebind of recording hotkey '{}' failed: {}",
                        old.hotkey, re
                    );
                }
            }
            return Err(format!(
                "Could not bind re-paste hotkey '{}': {}",
                settings.repaste_hotkey, e
            ));
        }
    }

    *state.settings.lock() = settings.clone();
    if let Err(e) = settings.save(&state.app_dir) {
        warn!("Settings applied in memory but disk save failed: {}", e);
        return Err(e);
    }

    if recording_changed {
        info!("Recording hotkey: {} â†’ {}", old.hotkey, settings.hotkey);
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            if let Err(e) = tray.set_tooltip(Some(tray_tooltip(&settings.hotkey))) {
                warn!("Could not update tray tooltip: {}", e);
            }
        }
    }
    if panel_changed {
        info!(
            "Settings hotkey: {} â†’ {}",
            old.settings_hotkey, settings.settings_hotkey
        );
    }
    if repaste_changed {
        info!(
            "Re-paste hotkey: {:?} â†’ {:?}",
            old.repaste_hotkey, settings.repaste_hotkey
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
    tauri::async_runtime::spawn_blocking(move || audio::test_microphone(&name, 1500))
        .await
        .map_err(|e| format!("Test thread panicked: {}", e))?
}

#[tauri::command]
fn open_mic_privacy_settings() -> Result<(), String> {
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

#[tauri::command]
fn get_history(state: State<AppState>) -> Vec<HistoryEntry> {
    state.history.lock().entries.clone()
}

#[tauri::command]
fn clear_history(state: State<AppState>) -> Result<(), String> {
    {
        let mut h = state.history.lock();
        h.clear();
        h.save(&state.app_dir);
    }
    Ok(())
}

async fn handle_hotkey_event(
    app: &AppHandle,
    state: &AppState,
    pressed: bool,
) -> Result<&'static str, String> {
    let mode = state.settings.lock().recording_mode.clone();

    match mode.as_str() {
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
            _ => Ok("ptt: noop"),
        },

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
    if handle
        .state::<AppState>()
        .registered_hotkeys
        .lock()
        .contains(accelerator)
    {
        return Ok(());
    }
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
                        if e.contains("transcribing") {
                            return;
                        }
                        error!("Hotkey failed: {}", e);
                        let _ = handle.emit("recording-error", &e);
                    }
                }
            });
        })
        .map_err(|e| e.to_string())?;
    handle
        .state::<AppState>()
        .registered_hotkeys
        .lock()
        .insert(accelerator.to_string());
    Ok(())
}

fn unregister_hotkey(handle: &AppHandle, accelerator: &str) {
    if accelerator.is_empty() {
        return;
    }
    if let Err(e) = handle.global_shortcut().unregister(accelerator) {
        warn!("Failed to unregister hotkey '{}': {}", accelerator, e);
    }
    handle
        .state::<AppState>()
        .registered_hotkeys
        .lock()
        .remove(accelerator);
}

fn rebind_recording_hotkey(handle: &AppHandle, old: &str, new: &str) -> Result<(), String> {
    register_recording_hotkey(handle, new)?;
    if !old.is_empty() && old != new {
        unregister_hotkey(handle, old);
    }
    Ok(())
}

fn register_settings_hotkey(handle: &AppHandle, accelerator: &str) -> Result<(), String> {
    if accelerator.is_empty() {
        return Ok(());
    }
    if handle
        .state::<AppState>()
        .registered_hotkeys
        .lock()
        .contains(accelerator)
    {
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
        .map_err(|e| e.to_string())?;
    handle
        .state::<AppState>()
        .registered_hotkeys
        .lock()
        .insert(accelerator.to_string());
    Ok(())
}

fn toggle_settings_window(handle: &AppHandle) {
    let Some(w) = handle.get_webview_window("settings") else {
        warn!("toggle_settings_window: settings window not found");
        return;
    };
    let state = handle.state::<AppState>();

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
    register_settings_hotkey(handle, new)?;
    if !old.is_empty() && old != new {
        unregister_hotkey(handle, old);
    }
    Ok(())
}

fn register_repaste_hotkey(handle: &AppHandle, accelerator: &str) -> Result<(), String> {
    if accelerator.is_empty() {
        return Ok(());
    }
    if handle
        .state::<AppState>()
        .registered_hotkeys
        .lock()
        .contains(accelerator)
    {
        return Ok(());
    }
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
                let (text, restore) = {
                    let h = state.history.lock();
                    let Some(latest) = h.latest() else {
                        let _ = handle.emit(
                            "recording-error",
                            "Nothing to re-paste yet â€” dictate something first.",
                        );
                        return;
                    };
                    (latest.text.clone(), state.settings.lock().clipboard_restore)
                };
                let result = tauri::async_runtime::spawn_blocking(move || {
                    paste_text(&text, restore)
                })
                .await
                .unwrap_or_else(|e| Err(format!("Repaste thread panicked: {}", e)));
                match result {
                    Ok(()) => {
                        info!("Hotkey: re-paste: pasted latest entry");
                        let _ = handle.emit("recording-state", "done");
                    }
                    Err(e) => {
                        error!("Re-paste failed: {}", e);
                        let _ = handle.emit("recording-error", &e);
                    }
                }
            });
        })
        .map_err(|e| e.to_string())?;
    handle
        .state::<AppState>()
        .registered_hotkeys
        .lock()
        .insert(accelerator.to_string());
    Ok(())
}

fn rebind_repaste_hotkey(handle: &AppHandle, old: &str, new: &str) -> Result<(), String> {
    register_repaste_hotkey(handle, new)?;
    if !old.is_empty() && old != new {
        unregister_hotkey(handle, old);
    }
    Ok(())
}

fn overlay_position(app: &AppHandle) -> (f64, f64) {
    if let Ok(Some(m)) = app.primary_monitor() {
        let scale = m.scale_factor();
        let logical_w = m.size().width as f64 / scale;
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
    ducking::restore_pending(&dir.join("pre_duck.txt"));
    let settings = Settings::load(&dir);
    let initial_hotkey = settings.hotkey.clone();
    let initial_settings_hotkey = settings.settings_hotkey.clone();
    let initial_repaste_hotkey = settings.repaste_hotkey.clone();
    let api_key_missing = settings.groq_api_key.trim().is_empty();
    let initial_tray_tooltip = tray_tooltip(&initial_hotkey);

    let history = Arc::new(Mutex::new(History::load(&dir)));

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            recorder: Recorder::new(&dir, history.clone()),
            settings: Mutex::new(settings),
            app_dir: dir,
            last_settings_toggle: Mutex::new(None),
            registered_hotkeys: Mutex::new(HashSet::new()),
            history,
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            list_microphones,
            test_microphone,
            open_mic_privacy_settings,
            open_settings,
            quit,
            get_history,
            clear_history,
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

            if let Some(settings_win) = handle.get_webview_window("settings") {
                let win = settings_win.clone();
                settings_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                });
            } else {
                warn!("Settings window not found at setup time â€” close-to-hide not wired");
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
                        "Recording hotkey {} couldn't be bound â€” another app may already use it. Pick a different combination in Settings â†’ Hotkey.",
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
                        "Settings hotkey {} couldn't be bound â€” another app may already use it. Pick a different combination in Settings â†’ Hotkey.",
                        initial_settings_hotkey
                    ),
                );
            }

            if !initial_repaste_hotkey.is_empty() {
                info!("Registering re-paste hotkey: {}", initial_repaste_hotkey);
                if let Err(e) = register_repaste_hotkey(&handle, &initial_repaste_hotkey) {
                    warn!(
                        "Failed to register re-paste hotkey '{}': {}",
                        initial_repaste_hotkey, e
                    );
                    let _ = handle.emit(
                        "recording-error",
                        format!(
                            "Re-paste hotkey {} couldn't be bound â€” another app may already use it. Pick a different combination in Settings â†’ Hotkey.",
                            initial_repaste_hotkey
                        ),
                    );
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
