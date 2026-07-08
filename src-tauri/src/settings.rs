use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, path::PathBuf};

const KEYRING_SERVICE: &str = "com.airwrite.app";
const KEYRING_USER: &str = "groq-api-key";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub microphone: String,
    #[serde(rename = "groqApiKey")]
    pub groq_api_key: String,
    #[serde(rename = "recordingMode")]
    pub recording_mode: String,
    pub hotkey: String,
    #[serde(rename = "settingsHotkey")]
    pub settings_hotkey: String,
    #[serde(rename = "duckingEnabled")]
    pub ducking_enabled: bool,
    #[serde(rename = "duckingLevel")]
    pub ducking_level: u8,
    #[serde(rename = "aiCleanupEnabled")]
    pub ai_cleanup_enabled: bool,
    #[serde(rename = "clipboardRestore")]
    pub clipboard_restore: bool,
    #[serde(rename = "repasteHotkey")]
    pub repaste_hotkey: String,
    #[serde(rename = "transcriptionLanguage")]
    pub transcription_language: String,
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
            ai_cleanup_enabled: false,
            clipboard_restore: true,
            repaste_hotkey: String::new(),
            transcription_language: "en".to_string(),
        }
    }
}

impl Settings {
    pub fn config_path(app_dir: &Path) -> PathBuf {
        app_dir.join("config.json")
    }

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

        match keyring_get() {
            Ok(Some(k)) => settings.groq_api_key = k,
            Ok(None) => {}
            Err(e) => warn!("Could not read API key from Credential Manager: {}", e),
        }

        settings
    }

    pub fn save(&self, app_dir: &Path) -> Result<(), String> {
        let trimmed = self.groq_api_key.trim();
        if trimmed.is_empty() {
            if let Err(e) = keyring_delete() {
                warn!("Could not delete API key from Credential Manager: {}", e);
            }
        } else {
            keyring_set(trimmed).map_err(|e| format!("Failed to save API key: {}", e))?;
        }
        self.save_config_only(app_dir)
    }

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
