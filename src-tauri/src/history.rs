//! Bounded transcription history persisted next to `config.json`.
//!
//! Stores the most recent N successful dictations so the user can re-paste
//! one (when the original paste landed in the wrong window) or scan what
//! they've said today. Capped at 20 entries on disk; older ones are
//! evicted on each `push`.
//!
//! Failure to read or write the file is non-fatal: we'd rather lose history
//! than block a recording. Corrupt files are replaced silently with an empty
//! history on next launch.

use log::warn;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const HISTORY_FILENAME: &str = "history.json";
const MAX_ENTRIES: usize = 20;

/// One transcription. `timestamp` is Unix seconds; `duration_secs` is the
/// length of the audio that produced this text (so the UI can show
/// "5.4s · 47 words"). Word count is computed at push time so the UI
/// doesn't have to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub text: String,
    pub timestamp: u64,
    pub duration_secs: f32,
    pub word_count: usize,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct History {
    /// Newest first.
    pub entries: Vec<HistoryEntry>,
}

impl History {
    fn path(app_dir: &Path) -> PathBuf {
        app_dir.join(HISTORY_FILENAME)
    }

    /// Read history from disk. Missing or unreadable file → empty history;
    /// corrupt JSON also → empty history (with a warn log, no error to the
    /// user).
    pub fn load(app_dir: &Path) -> Self {
        let path = Self::path(app_dir);
        match std::fs::read_to_string(&path) {
            Ok(s) => match serde_json::from_str::<History>(&s) {
                Ok(h) => h,
                Err(e) => {
                    warn!(
                        "Could not parse {}: {}. Starting with empty history.",
                        path.display(),
                        e
                    );
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                warn!("Could not read {}: {}.", path.display(), e);
                Self::default()
            }
        }
    }

    /// Persist to disk. Errors logged but never propagated — losing a
    /// history write must not block the user's pipeline.
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
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    warn!("Could not write {}: {}", path.display(), e);
                }
            }
            Err(e) => warn!("Could not serialise history: {}", e),
        }
    }

    /// Insert a new entry at the front; evict from the back to stay within
    /// `MAX_ENTRIES`. Empty / whitespace-only text is dropped — no point
    /// remembering nothing.
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

    pub fn get(&self, index: usize) -> Option<&HistoryEntry> {
        self.entries.get(index)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
