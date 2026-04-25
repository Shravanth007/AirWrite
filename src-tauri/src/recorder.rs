use log::info;
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use crate::audio::AudioRecorder;
use crate::cleanup::cleanup_text;
use crate::paste::paste_text;
use crate::settings::Settings;
use crate::transcribe_groq;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum RecordingState {
    Ready,
    Recording,
    Transcribing,
}

pub struct Recorder {
    state: Arc<Mutex<RecordingState>>,
    audio_recorder: Arc<Mutex<AudioRecorder>>,
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}

impl Recorder {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(RecordingState::Ready)),
            audio_recorder: Arc::new(Mutex::new(AudioRecorder::new())),
        }
    }

    pub fn get_state(&self) -> RecordingState {
        self.state.lock().clone()
    }

    pub fn start_recording(&self, app: &AppHandle, mic_name: &str) -> Result<(), String> {
        {
            let mut state = self.state.lock();
            if *state != RecordingState::Ready {
                return Err("Already recording or transcribing".to_string());
            }
            self.audio_recorder.lock().start(mic_name)?;
            *state = RecordingState::Recording;
        }
        let _ = app.emit("recording-state", "recording");
        Ok(())
    }

    pub async fn stop_and_transcribe(
        &self,
        app: &AppHandle,
        settings: &Settings,
    ) -> Result<String, String> {
        {
            let mut state = self.state.lock();
            if *state != RecordingState::Recording {
                return Err("Not currently recording".to_string());
            }
            *state = RecordingState::Transcribing;
        }
        let _ = app.emit("recording-state", "transcribing");

        let result = self.do_transcribe(settings).await;

        // Always reset state, regardless of outcome.
        *self.state.lock() = RecordingState::Ready;

        match &result {
            Ok(text) => {
                info!("Transcription: {:?}", text);
                let _ = app.emit("recording-state", "done");
                let _ = app.emit("recording-transcription", text);
            }
            Err(e) => {
                let _ = app.emit("recording-error", e);
            }
        }
        result
    }

    async fn do_transcribe(&self, settings: &Settings) -> Result<String, String> {
        // NamedTempFile auto-deletes when dropped — survives panic and crash.
        let temp = tempfile::Builder::new()
            .prefix("airwrite-")
            .suffix(".wav")
            .tempfile()
            .map_err(|e| format!("Failed to create temp file: {}", e))?;
        let temp_path = temp.path().to_path_buf();

        self.audio_recorder.lock().stop_and_save(&temp_path)?;

        let api_key = settings.groq_api_key.trim().to_string();
        let raw_text = transcribe_groq::transcribe_groq(&api_key, &temp_path).await?;
        // `temp` drops here → file is removed.
        drop(temp);

        let cleaned = cleanup_text(&raw_text);
        if !cleaned.is_empty() {
            paste_text(&cleaned)?;
        }
        Ok(cleaned)
    }
}
