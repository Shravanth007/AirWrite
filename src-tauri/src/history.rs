
use log::warn;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const HISTORY_FILENAME: &str = "history.json";
const MAX_ENTRIES: usize = 20;

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
