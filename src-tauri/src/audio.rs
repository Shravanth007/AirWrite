use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat};
use hound::{WavSpec, WavWriter};
use log::{debug, error, info, warn};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;

/// Peak below this threshold is treated as "no real audio" — Whisper
/// hallucinates phrases like "Thank you" on near-silent input, so we surface
/// a clean error instead of pasting nonsense.
const SILENCE_PEAK_THRESHOLD: f32 = 0.005; // ≈ -46 dBFS

#[derive(Debug, Clone, serde::Serialize)]
pub struct MicDevice {
    pub name: String,
    pub is_default: bool,
}

pub fn list_microphones() -> Vec<MicDevice> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();

    let mut devices = Vec::new();
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                devices.push(MicDevice {
                    is_default: name == default_name,
                    name,
                });
            }
        }
    }
    devices
}

/// Wrapper to hold a `cpal::Stream` inside `AudioRecorder` (shared via
/// `Arc<Mutex<…>>`). On Windows the underlying WASAPI handle is COM-affine:
/// it must be created and dropped on the same thread. We satisfy that today
/// because `start` and `stop_and_save` both run on the global-shortcut
/// callback's spawned task, which Tauri pins to a single async runtime
/// thread.
struct SendStream(#[allow(dead_code)] cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    stream: Option<SendStream>,
    source_sample_rate: u32,
    source_channels: u16,
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            source_sample_rate: 48000,
            source_channels: 1,
        }
    }

    pub fn start(&mut self, mic_name: &str) -> Result<(), String> {
        self.samples.lock().clear();

        let host = cpal::default_host();
        let device = if mic_name == "default" {
            host.default_input_device()
                .ok_or("No default input device found")?
        } else {
            host.input_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().map(|n| n == mic_name).unwrap_or(false))
                .ok_or_else(|| format!("Microphone '{}' not found", mic_name))?
        };

        let device_label = device.name().unwrap_or_else(|_| "<unknown>".into());
        let supported = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let sample_format = supported.sample_format();
        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels();
        info!(
            "Opening mic '{}': {}Hz, {} ch, format {:?}",
            device_label, sample_rate, channels, sample_format
        );

        self.source_sample_rate = sample_rate;
        self.source_channels = channels;

        let config: cpal::StreamConfig = supported.config();
        let samples = self.samples.clone();
        let err_fn = |e: cpal::StreamError| error!("Audio stream error: {}", e);

        // CPAL's typed `build_input_stream::<T>` uses the closure's element type
        // to negotiate the format with the OS. Picking the wrong T silently
        // produces garbage on some Windows drivers, which is what causes
        // Whisper to return "Thank you" on otherwise-working systems. Branch
        // on the real format reported by the device.
        let stream = match sample_format {
            SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    samples.lock().extend_from_slice(data);
                },
                err_fn,
                None,
            ),
            SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let mut buf = samples.lock();
                    buf.extend(data.iter().map(|&s| s.to_sample::<f32>()));
                },
                err_fn,
                None,
            ),
            SampleFormat::U16 => device.build_input_stream(
                &config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let mut buf = samples.lock();
                    buf.extend(data.iter().map(|&s| s.to_sample::<f32>()));
                },
                err_fn,
                None,
            ),
            SampleFormat::I32 => device.build_input_stream(
                &config,
                move |data: &[i32], _: &cpal::InputCallbackInfo| {
                    let mut buf = samples.lock();
                    buf.extend(data.iter().map(|&s| s.to_sample::<f32>()));
                },
                err_fn,
                None,
            ),
            other => return Err(format!("Unsupported sample format: {:?}", other)),
        }
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start mic stream: {}", e))?;
        self.stream = Some(SendStream(stream));
        debug!("Audio recording started");
        Ok(())
    }

    /// Stop the active stream, write the captured audio to `output_path`,
    /// and return the duration of the recording in seconds (so callers can
    /// reason about real-time-factor against the transcription latency).
    pub fn stop_and_save(&mut self, output_path: &Path) -> Result<f32, String> {
        // Drop stops the stream — must happen on the same thread as `start`.
        self.stream = None;
        debug!("Audio recording stopped");

        let mut samples = self.samples.lock();
        if samples.is_empty() {
            return Err(
                "No audio captured. Check that your microphone is enabled and not muted.".into(),
            );
        }

        let mono: Vec<f32> = if self.source_channels > 1 {
            samples
                .chunks(self.source_channels as usize)
                .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
                .collect()
        } else {
            samples.clone()
        };

        let duration_secs = mono.len() as f32 / self.source_sample_rate as f32;
        let peak = mono.iter().fold(0.0_f32, |a, &s| a.max(s.abs()));
        let rms = if mono.is_empty() {
            0.0
        } else {
            (mono.iter().map(|&s| s * s).sum::<f32>() / mono.len() as f32).sqrt()
        };
        info!(
            "Captured {:.2}s of audio: peak={:.4} ({:.1} dBFS), rms={:.4}",
            duration_secs,
            peak,
            20.0 * peak.max(1e-9).log10(),
            rms
        );

        if peak < SILENCE_PEAK_THRESHOLD {
            warn!("Audio peak {:.5} below silence threshold — refusing to send.", peak);
            samples.clear();
            return Err(format!(
                "Microphone captured silence (peak {:.4}). \
                 Check Windows mic permissions, mute switch, and that the right input is selected in Settings.",
                peak
            ));
        }

        let resampled = resample(&mono, self.source_sample_rate, 16000);
        debug!("Resampled to {} samples at 16kHz", resampled.len());

        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(output_path, spec).map_err(|e| e.to_string())?;
        for &sample in resampled.iter() {
            let amplitude = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(amplitude).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;
        samples.clear();

        debug!("WAV saved to {}", output_path.display());
        Ok(duration_secs)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MicTestResult {
    pub device: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub format: String,
    pub duration_ms: u32,
    pub samples_collected: usize,
    pub peak: f32,
    pub peak_db: f32,
    pub rms: f32,
    pub verdict: String,
}

/// Run a short capture and report level metrics. Used by Settings → "Test mic"
/// so the user can confirm Windows is actually feeding audio to the app.
pub fn test_microphone(mic_name: &str, duration_ms: u32) -> Result<MicTestResult, String> {
    let mut recorder = AudioRecorder::new();
    recorder.start(mic_name)?;
    std::thread::sleep(std::time::Duration::from_millis(duration_ms as u64));

    // Drop the stream; pull the captured samples.
    recorder.stream = None;
    let samples = std::mem::take(&mut *recorder.samples.lock());

    let host = cpal::default_host();
    let device = if mic_name == "default" {
        host.default_input_device()
    } else {
        host.input_devices()
            .ok()
            .and_then(|mut it| it.find(|d| d.name().map(|n| n == mic_name).unwrap_or(false)))
    };
    let device_label = device
        .as_ref()
        .and_then(|d| d.name().ok())
        .unwrap_or_else(|| mic_name.to_string());
    let format = device
        .as_ref()
        .and_then(|d| d.default_input_config().ok())
        .map(|c| format!("{:?}", c.sample_format()))
        .unwrap_or_else(|| "unknown".into());

    let mono: Vec<f32> = if recorder.source_channels > 1 {
        samples
            .chunks(recorder.source_channels as usize)
            .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
            .collect()
    } else {
        samples
    };

    let peak = mono.iter().fold(0.0_f32, |a, &s| a.max(s.abs()));
    let rms = if mono.is_empty() {
        0.0
    } else {
        (mono.iter().map(|&s| s * s).sum::<f32>() / mono.len() as f32).sqrt()
    };
    let peak_db = 20.0 * peak.max(1e-9).log10();

    let verdict = if mono.is_empty() {
        "No samples captured. Stream did not produce data.".to_string()
    } else if peak < 0.001 {
        "SILENT — Windows is not letting this app hear the mic. Check 'Let desktop apps access your microphone' in Privacy settings.".to_string()
    } else if peak < SILENCE_PEAK_THRESHOLD {
        "Very quiet — speak louder or check Levels in Windows sound panel.".to_string()
    } else if peak < 0.05 {
        "Audio detected but quiet. Speech may transcribe poorly. Boost mic Level in Windows.".to_string()
    } else {
        "Healthy signal — mic is working.".to_string()
    };

    Ok(MicTestResult {
        device: device_label,
        sample_rate: recorder.source_sample_rate,
        channels: recorder.source_channels,
        format,
        duration_ms,
        samples_collected: mono.len(),
        peak,
        peak_db,
        rms,
        verdict,
    })
}

/// Linear-interpolation resampler. Naive — no anti-aliasing filter — but
/// adequate for speech up to ~4 kHz, which is all Whisper cares about.
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);
    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx = src_idx as usize;
        let frac = src_idx - idx as f64;
        let sample = if idx + 1 < samples.len() {
            samples[idx] as f64 * (1.0 - frac) + samples[idx + 1] as f64 * frac
        } else {
            samples[idx.min(samples.len() - 1)] as f64
        };
        output.push(sample as f32);
    }
    output
}
