use log::{info, warn};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

use crate::audio::AudioRecorder;
use crate::cleanup::cleanup_text;
use crate::ducking;
use crate::history::History;
use crate::llm_cleanup;
use crate::paste::paste_text;
use crate::settings::Settings;
use crate::transcribe_groq;

const DUCK_RECOVERY_FILENAME: &str = "pre_duck.txt";

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum RecordingState {
    Ready,
    Recording,
    Transcribing,
}

pub struct Recorder {
    state: Arc<Mutex<RecordingState>>,
    audio_recorder: Arc<Mutex<AudioRecorder>>,
    pre_duck_volume: Arc<Mutex<Option<f32>>>,
    duck_recovery_path: PathBuf,
    history: Arc<Mutex<History>>,
    app_dir: PathBuf,
}

impl Recorder {
    pub fn new(app_dir: &Path, history: Arc<Mutex<History>>) -> Self {
        Self {
            state: Arc::new(Mutex::new(RecordingState::Ready)),
            audio_recorder: Arc::new(Mutex::new(AudioRecorder::new())),
            pre_duck_volume: Arc::new(Mutex::new(None)),
            duck_recovery_path: app_dir.join(DUCK_RECOVERY_FILENAME),
            history,
            app_dir: app_dir.to_path_buf(),
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

        if settings.ducking_enabled {
            match ducking::duck(settings.ducking_level) {
                Ok(prior) => {
                    *self.pre_duck_volume.lock() = Some(prior);
                    ducking::save_pending(prior, &self.duck_recovery_path);
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

    fn restore_volume(&self) {
        if let Some(prior) = self.pre_duck_volume.lock().take() {
            ducking::restore(prior);
            ducking::clear_pending(&self.duck_recovery_path);
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
        self.restore_volume();
        let _ = app.emit("recording-state", "transcribing");

        let result = self.do_transcribe(settings, app).await;

        *self.state.lock() = RecordingState::Ready;

        match &result {
            Ok((text, truncated)) => {
                info!("Transcription: {:?}", text);
                let _ = app.emit("recording-state", "done");
                if *truncated {
                    let _ = app.emit(
                        "recording-warning",
                        "Recording hit the 5-minute limit — only the first 5 minutes were transcribed.",
                    );
                }
            }
            Err(e) => {
                let _ = app.emit("recording-error", e);
            }
        }
        result.map(|(text, _)| text)
    }

    async fn do_transcribe(
        &self,
        settings: &Settings,
        app: &AppHandle,
    ) -> Result<(String, bool), String> {
        let temp = tempfile::Builder::new()
            .prefix("airwrite-")
            .suffix(".wav")
            .tempfile()
            .map_err(|e| format!("Failed to create temp file: {}", e))?;
        let temp_path = temp.path().to_path_buf();

        let pipeline_started = Instant::now();
        let (audio_secs, truncated) = self.audio_recorder.lock().stop_and_save(&temp_path)?;
        let upload_size = std::fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0);

        let api_key = settings.groq_api_key.trim().to_string();
        let api_started = Instant::now();
        let raw_text = transcribe_groq::transcribe_groq(&api_key, &temp_path).await?;
        let api_secs = api_started.elapsed().as_secs_f32();

        let cleaned = cleanup_text(&raw_text);

        let (final_text, llm_secs) = if settings.ai_cleanup_enabled && !cleaned.is_empty() {
            let llm_started = Instant::now();
            match llm_cleanup::cleanup_with_llm(&api_key, &cleaned).await {
                Ok(polished) => {
                    let secs = llm_started.elapsed().as_secs_f32();
                    info!("LLM cleanup ({:.2}s): {:?} → {:?}", secs, cleaned, polished);
                    (polished, Some(secs))
                }
                Err(e) => {
                    warn!("LLM cleanup failed, using raw transcription: {}", e);
                    (cleaned, None)
                }
            }
        } else {
            (cleaned, None)
        };

        let paste_started = Instant::now();
        if !final_text.is_empty() {
            paste_text(&final_text, settings.clipboard_restore)?;
        }
        let paste_secs = paste_started.elapsed().as_secs_f32();
        let total_secs = pipeline_started.elapsed().as_secs_f32();

        if !final_text.is_empty() {
            let mut h = self.history.lock();
            h.push(&final_text, audio_secs);
            h.save(&self.app_dir);
            drop(h);
            let _ = app.emit("history-updated", ());
        }

        let rtf = if audio_secs > 0.0 { api_secs / audio_secs } else { 0.0 };
        match llm_secs {
            Some(secs) => info!(
                "Speed: groq={:.2}s rtf={:.2}x · llm={:.2}s · audio={:.2}s · upload={:.0}KB · paste={:.2}s · total={:.2}s",
                api_secs,
                rtf,
                secs,
                audio_secs,
                upload_size as f32 / 1024.0,
                paste_secs,
                total_secs,
            ),
            None => info!(
                "Speed: groq={:.2}s rtf={:.2}x · audio={:.2}s · upload={:.0}KB · paste={:.2}s · total={:.2}s",
                api_secs,
                rtf,
                audio_secs,
                upload_size as f32 / 1024.0,
                paste_secs,
                total_secs,
            ),
        }

        Ok((final_text, truncated))
    }
}
