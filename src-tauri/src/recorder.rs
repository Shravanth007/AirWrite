use log::{info, warn};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

use crate::audio::AudioRecorder;
use crate::cleanup::cleanup_text;
use crate::ducking;
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
    /// Master output volume captured right before we ducked it. `Some` only
    /// while a duck is in effect; cleared on restore. Lives on the Recorder
    /// so the duck/restore pair is naturally tied to the recording lifetime.
    pre_duck_volume: Arc<Mutex<Option<f32>>>,
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
            pre_duck_volume: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_state(&self) -> RecordingState {
        self.state.lock().clone()
    }

    pub fn start_recording(
        &self,
        app: &AppHandle,
        settings: &Settings,
    ) -> Result<(), String> {
        {
            let mut state = self.state.lock();
            if *state != RecordingState::Ready {
                return Err("Already recording or transcribing".to_string());
            }
            self.audio_recorder.lock().start(&settings.microphone)?;
            *state = RecordingState::Recording;
        }

        // Apply ducking AFTER the mic is already capturing. Failure is
        // non-fatal — the recording is more important than the duck.
        if settings.ducking_enabled {
            match ducking::duck(settings.ducking_level) {
                Ok(prior) => {
                    *self.pre_duck_volume.lock() = Some(prior);
                    info!(
                        "Ducked master volume: {:.0}% → {}%",
                        prior * 100.0,
                        settings.ducking_level
                    );
                }
                Err(e) => warn!("Audio ducking failed (continuing): {}", e),
            }
        }

        let _ = app.emit("recording-state", "recording");
        Ok(())
    }

    /// Restore the master volume if we ducked. Idempotent — clears the
    /// snapshot so subsequent calls are no-ops. Always called on the path
    /// out of a recording, regardless of success/failure.
    fn restore_volume(&self) {
        if let Some(prior) = self.pre_duck_volume.lock().take() {
            ducking::restore(prior);
        }
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
        // Restore audio as soon as the user stops recording, not after
        // transcription finishes — they want the music back the moment they
        // release the key, not after Groq replies.
        self.restore_volume();
        let _ = app.emit("recording-state", "transcribing");

        let result = self.do_transcribe(settings).await;

        // Always reset state, regardless of outcome.
        *self.state.lock() = RecordingState::Ready;

        match &result {
            Ok(text) => {
                info!("Transcription: {:?}", text);
                let _ = app.emit("recording-state", "done");
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

        let pipeline_started = Instant::now();
        let audio_secs = self.audio_recorder.lock().stop_and_save(&temp_path)?;
        let upload_size = std::fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0);

        let api_key = settings.groq_api_key.trim().to_string();
        let api_started = Instant::now();
        let raw_text = transcribe_groq::transcribe_groq(&api_key, &temp_path).await?;
        let api_secs = api_started.elapsed().as_secs_f32();
        // `temp` drops here → file is removed.
        drop(temp);

        let cleaned = cleanup_text(&raw_text);
        let paste_started = Instant::now();
        if !cleaned.is_empty() {
            paste_text(&cleaned)?;
        }
        let paste_secs = paste_started.elapsed().as_secs_f32();
        let total_secs = pipeline_started.elapsed().as_secs_f32();

        // Real-time factor: how long Groq took relative to the audio duration.
        // <1.0 means "faster than real-time" (typical for whisper-large-v3-turbo
        // on Groq, often 0.05–0.20). >1.0 means the network or the model is
        // bottlenecking.
        let rtf = if audio_secs > 0.0 { api_secs / audio_secs } else { 0.0 };
        info!(
            "Speed: groq={:.2}s rtf={:.2}x · audio={:.2}s · upload={:.0}KB · paste={:.2}s · total={:.2}s",
            api_secs,
            rtf,
            audio_secs,
            upload_size as f32 / 1024.0,
            paste_secs,
            total_secs,
        );

        Ok(cleaned)
    }
}
