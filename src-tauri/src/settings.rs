use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, path::PathBuf};

const KEYRING_SERVICE: &str = "com.airwrite.app";
const KEYRING_USER: &str = "groq-api-key";

/// Persisted, non-secret settings. The Groq API key is NOT stored here —
/// it lives in Windows Credential Manager via the `keyring` crate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub microphone: String,
    #[serde(rename = "groqApiKey")]
    pub groq_api_key: String,
    #[serde(rename = "recordingMode")]
    pub recording_mode: String,
    pub hotkey: String,
    /// Global hotkey that shows/focuses the Settings window.
    #[serde(rename = "settingsHotkey")]
    pub settings_hotkey: String,
    /// Whether to lower the Windows master output volume while recording.
    #[serde(rename = "duckingEnabled")]
    pub ducking_enabled: bool,
    /// Target level (0–100) the master volume is *reduced to* while
    /// recording. Lower = quieter background. 0 mutes other audio entirely.
    #[serde(rename = "duckingLevel")]
    pub ducking_level: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            microphone: "default".to_string(),
            groq_api_key: String::new(),
            recording_mode: "toggle".to_string(),
            hotkey: "CmdOrCtrl+Shift+Space".to_string(),
            settings_hotkey: "CmdOrCtrl+Alt+S".to_string(),
            ducking_enabled: true,
            ducking_level: 15,
        }
    }
}

impl Settings {
    pub fn config_path(app_dir: &Path) -> PathBuf {
        app_dir.join("config.json")
    }

    /// Load settings from disk, then hydrate `groq_api_key` from the OS keychain.
    /// Migrates a plaintext key found in `config.json` (from older builds) into
    /// the keychain and rewrites the config with an empty `groqApiKey`.
    pub fn load(app_dir: &Path) -> Self {
        let path = Self::config_path(app_dir);
        let mut settings: Settings = match fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str(&contents) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to parse {}: {}. Backing up and using defaults.", path.display(), e);
                    let backup = path.with_extension("json.bad");
                    if let Err(be) = fs::rename(&path, &backup) {
                        warn!("Could not back up corrupt config to {}: {}", backup.display(), be);
                    }
                    Settings::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(e) => {
                warn!("Could not read {}: {}. Using defaults.", path.display(), e);
                Settings::default()
            }
        };

        // Migration: if a plaintext key from an older build is in config.json,
        // move it to the keychain and clear the field on disk.
        let plaintext = std::mem::take(&mut settings.groq_api_key);
        if !plaintext.trim().is_empty() {
            match keyring_set(plaintext.trim()) {
                Ok(_) => {
                    info!("Migrated plaintext Groq API key from config.json into Credential Manager.");
                    if let Err(e) = settings.save_config_only(app_dir) {
                        warn!("Failed to rewrite config.json after migration: {}", e);
                    }
                }
                Err(e) => {
                    warn!("Failed to migrate API key to keychain: {}. Keeping in memory only.", e);
                    settings.groq_api_key = plaintext;
                    return settings;
                }
            }
        }

        // Hydrate API key from keychain.
        match keyring_get() {
            Ok(Some(k)) => settings.groq_api_key = k,
            Ok(None) => {}
            Err(e) => warn!("Could not read API key from Credential Manager: {}", e),
        }

        settings
    }

    /// Persist settings: API key → keychain, everything else → config.json.
    pub fn save(&self, app_dir: &Path) -> Result<(), String> {
        let trimmed = self.groq_api_key.trim();
        if trimmed.is_empty() {
            // Empty key means "clear it" — remove from keychain so a stale key
            // doesn't linger after the user blanks the field.
            if let Err(e) = keyring_delete() {
                warn!("Could not delete API key from Credential Manager: {}", e);
            }
        } else {
            keyring_set(trimmed).map_err(|e| format!("Failed to save API key: {}", e))?;
        }
        self.save_config_only(app_dir)
    }

    /// Write only the JSON file (does not touch the keychain). Used internally
    /// during migration and after `save` has handled the secret.
    fn save_config_only(&self, app_dir: &Path) -> Result<(), String> {
        let path = Self::config_path(app_dir);
        fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
        let on_disk = Settings {
            groq_api_key: String::new(),
            ..self.clone()
        };
        let json = serde_json::to_string_pretty(&on_disk).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())
    }
}

fn keyring_entry() -> keyring::Result<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
}

fn keyring_get() -> keyring::Result<Option<String>> {
    match keyring_entry()?.get_password() {
        Ok(s) => Ok(Some(s)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e),
    }
}

fn keyring_set(secret: &str) -> keyring::Result<()> {
    keyring_entry()?.set_password(secret)
}

fn keyring_delete() -> keyring::Result<()> {
    match keyring_entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e),
    }
}
