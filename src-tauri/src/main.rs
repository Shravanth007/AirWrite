#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod cleanup;
mod paste;
mod recorder;
mod settings;
mod transcribe_groq;

use recorder::{Recorder, RecordingState};
use settings::Settings;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

struct AppState {
    recorder: Recorder,
    settings: Mutex<Settings>,
    app_dir: PathBuf,
}

fn get_app_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.airwrite.app")
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    settings.save(&state.app_dir)?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
fn list_microphones() -> Vec<audio::MicDevice> {
    audio::list_microphones()
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

async fn do_toggle_recording(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<String, String> {
    match state.recorder.get_state() {
        RecordingState::Ready => {
            let mic = state.settings.lock().unwrap().microphone.clone();
            state.recorder.start_recording(app, &mic)?;
            if let Some(w) = app.get_webview_window("overlay") {
                let _ = w.show();
            }
            Ok("recording".to_string())
        }
        RecordingState::Recording => {
            let settings = state.settings.lock().unwrap().clone();
            let result = state
                .recorder
                .stop_and_transcribe(app, &settings, &state.app_dir)
                .await?;
            // Hide overlay shortly after "done" state is shown
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;
                if let Some(w) = app_clone.get_webview_window("overlay") {
                    let _ = w.hide();
                }
            });
            Ok(result)
        }
        RecordingState::Transcribing => {
            Err("Currently transcribing, please wait".to_string())
        }
    }
}

fn main() {
    let app_dir = get_app_dir();
    let settings = Settings::load(&app_dir);
    let initial_hotkey = settings.hotkey.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            recorder: Recorder::new(),
            settings: Mutex::new(settings),
            app_dir,
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            list_microphones,
            open_settings,
        ])
        .setup(move |app| {
            // ── System tray ───────────────────────────────────────────────
            {
                use tauri::menu::{MenuBuilder, MenuItemBuilder};
                use tauri::tray::TrayIconBuilder;

                let settings_item =
                    MenuItemBuilder::new("Settings").id("settings").build(app)?;
                let quit_item = MenuItemBuilder::new("Quit").id("quit").build(app)?;
                let menu = MenuBuilder::new(app)
                    .items(&[&settings_item, &quit_item])
                    .build()?;

                TrayIconBuilder::new()
                    .icon(app.default_window_icon().unwrap().clone())
                    .tooltip("AirWrite — Ctrl+Shift+Space to dictate")
                    .menu(&menu)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "settings" => open_settings(app.clone()),
                        "quit" => std::process::exit(0),
                        _ => {}
                    })
                    .build(app)?;
            }

            // ── Floating overlay window ────────────────────────────────────
            // Positioned top-right; focused(false) keeps the active window focused
            {
                let monitor = app.primary_monitor().ok().flatten();
                let (x, y) = if let Some(m) = monitor {
                    let size = m.size();
                    let scale = m.scale_factor();
                    let logical_w = size.width as f64 / scale;
                    ((logical_w - 320.0) as i32, 20_i32)
                } else {
                    (1580, 20)
                };

                match WebviewWindowBuilder::new(
                    app,
                    "overlay",
                    WebviewUrl::App("/".into()),
                )
                .title("")
                .inner_size(300.0, 52.0)
                .position(x as f64, y as f64)
                .resizable(false)
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .focused(false)
                .shadow(false)
                .visible(false)
                .build()
                {
                    Ok(_) => println!("[AirWrite] Overlay window created"),
                    Err(e) => eprintln!("[AirWrite] Failed to create overlay: {}", e),
                }
            }

            // ── First-run: open settings if no API key ─────────────────────
            if Settings::load(&get_app_dir()).groq_api_key.is_empty() {
                open_settings(app.handle().clone());
            }

            // ── Global hotkey ──────────────────────────────────────────────
            let handle = app.handle().clone();
            println!("[AirWrite] Registering hotkey: {}", initial_hotkey);

            match app.global_shortcut().on_shortcut(
                initial_hotkey.as_str(),
                move |_app, _shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let handle = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = handle.state::<AppState>();
                        match do_toggle_recording(&handle, state.inner()).await {
                            Ok(r) => println!("[AirWrite] {}", r),
                            Err(e) => {
                                eprintln!("[AirWrite] Error: {}", e);
                                let _ = handle.emit("recording-error", &e);
                                // Hide overlay after showing error briefly
                                let h = handle.clone();
                                tauri::async_runtime::spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(2000))
                                        .await;
                                    if let Some(w) = h.get_webview_window("overlay") {
                                        let _ = w.hide();
                                    }
                                });
                            }
                        }
                    });
                },
            ) {
                Ok(_) => println!("[AirWrite] Hotkey registered successfully"),
                Err(e) => eprintln!("[AirWrite] Failed to register hotkey: {}", e),
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
