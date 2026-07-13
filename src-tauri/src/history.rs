
use log::warn;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB};

const HISTORY_FILENAME: &str = "history.json";
const MAX_ENTRIES: usize = 20;

/// Encrypts bytes for the current Windows user via DPAPI (CryptProtectData,
/// current-user scope) so only this Windows account can decrypt history.json.
fn dpapi_encrypt(data: &[u8]) -> Result<Vec<u8>, String> {
    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        CryptProtectData(&input, PCWSTR::null(), None, None, None, 0, &mut output)
            .map_err(|e| e.to_string())?;
        let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(output.pbData as *mut _));
        Ok(result)
    }
}

/// Decrypts bytes previously produced by dpapi_encrypt (current Windows user only).
fn dpapi_decrypt(data: &[u8]) -> Result<Vec<u8>, String> {
    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        CryptUnprotectData(&input, None, None, None, None, 0, &mut output)
            .map_err(|e| e.to_string())?;
        let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(output.pbData as *mut _));
        Ok(result)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub text: String,
    pub timestamp: u64,
    pub duration_secs: f32,
    pub word_count: usize,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct History {
    pub entries: Vec<HistoryEntry>,
}

impl History {
    fn path(app_dir: &Path) -> PathBuf {
        app_dir.join(HISTORY_FILENAME)
    }

    pub fn load(app_dir: &Path) -> Self {
        let path = Self::path(app_dir);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                warn!("Could not read {}: {}.", path.display(), e);
                return Self::default();
            }
        };

        // Current format: DPAPI-encrypted JSON.
        if let Ok(plain) = dpapi_decrypt(&bytes) {
            match serde_json::from_slice::<History>(&plain) {
                Ok(h) => return h,
                Err(e) => warn!("Could not parse decrypted {}: {}.", path.display(), e),
            }
        } else if let Ok(h) = serde_json::from_slice::<History>(&bytes) {
            // Migration: pre-encryption plaintext history.json. Adopt it, then
            // rewrite it encrypted so it isn't left as plaintext on disk.
            warn!("Migrating plaintext {} to DPAPI-encrypted format.", path.display());
            h.save(app_dir);
            return h;
        }

        warn!(
            "Could not decrypt or parse {}. Backing up and starting with empty history.",
            path.display()
        );
        let backup = path.with_extension("json.bad");
        if let Err(be) = std::fs::rename(&path, &backup) {
            warn!("Could not back up corrupt history to {}: {}", backup.display(), be);
        }
        Self::default()
    }

    pub fn save(&self, app_dir: &Path) {
        let path = Self::path(app_dir);
        if let Err(e) = std::fs::create_dir_all(app_dir) {
            warn!(
                "Could not create app dir {} for history: {}",
                app_dir.display(),
                e
            );
            return;
        }
        let json = match serde_json::to_vec(self) {
            Ok(j) => j,
            Err(e) => {
                warn!("Could not serialise history: {}", e);
                return;
            }
        };
        match dpapi_encrypt(&json) {
            Ok(encrypted) => {
                if let Err(e) = std::fs::write(&path, encrypted) {
                    warn!("Could not write {}: {}", path.display(), e);
                }
            }
            Err(e) => warn!("Could not encrypt history for {}: {}", path.display(), e),
        }
    }

    pub fn push(&mut self, text: &str, duration_secs: f32) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let word_count = trimmed.split_whitespace().count();
        self.entries.insert(
            0,
            HistoryEntry {
                text: trimmed.to_string(),
                timestamp,
                duration_secs,
                word_count,
            },
        );
        if self.entries.len() > MAX_ENTRIES {
            self.entries.truncate(MAX_ENTRIES);
        }
    }

    pub fn latest(&self) -> Option<&HistoryEntry> {
        self.entries.first()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpapi_roundtrip() {
        let data = b"hello dpapi";
        let encrypted = dpapi_encrypt(data).unwrap();
        assert_ne!(encrypted, data);
        let decrypted = dpapi_decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut h = History::default();
        h.push("secret password 123", 1.5);
        h.save(dir.path());

        let loaded = History::load(dir.path());
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].text, "secret password 123");

        let raw = std::fs::read(History::path(dir.path())).unwrap();
        assert!(!raw.windows(6).any(|w| w == b"secret"));
    }

    #[test]
    fn migrates_plaintext_history() {
        let dir = tempfile::tempdir().unwrap();
        let mut h = History::default();
        h.push("old plaintext entry", 2.0);
        let json = serde_json::to_vec(&h).unwrap();
        std::fs::write(History::path(dir.path()), &json).unwrap();

        let loaded = History::load(dir.path());
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].text, "old plaintext entry");

        // load() should have rewritten the file encrypted.
        let raw = std::fs::read(History::path(dir.path())).unwrap();
        assert_ne!(raw, json);
    }
}
