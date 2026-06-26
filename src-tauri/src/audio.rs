use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat};
use hound::{WavSpec, WavWriter};
use log::{debug, error, info, warn};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

const SILENCE_PEAK_THRESHOLD: f32 = 0.005;
// ponytail: hard cap bounds in-memory sample buffer and prevents Groq HTTP 413;
//           raise MAX_RECORDING_SECS if longer dictations are ever needed.
const MAX_RECORDING_SECS: u32 = 300;

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

struct ActiveStream {
    _stream: cpal::Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    sample_format: SampleFormat,
}

pub struct DrainedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: SampleFormat,
}

enum AudioCommand {
    Start {
        mic_name: String,
        reply: Sender<Result<(), String>>,
    },
    Stop {
        reply: Sender<Result<DrainedAudio, String>>,
    },
    Shutdown,
}

pub struct AudioRecorder {
    cmd_tx: Sender<AudioCommand>,
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioRecorder {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(AudioCommand::Shutdown);
    }
}

impl AudioRecorder {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = channel();
        std::thread::Builder::new()
            .name("airwrite-audio".into())
            .spawn(move || worker_loop(cmd_rx))
            .expect("failed to spawn audio worker thread");
        Self { cmd_tx }
    }

    pub fn start(&mut self, mic_name: &str) -> Result<(), String> {
        let (reply_tx, reply_rx) = channel();
        self.cmd_tx
            .send(AudioCommand::Start {
                mic_name: mic_name.to_string(),
                reply: reply_tx,
            })
            .map_err(|_| "Audio worker thread is not running".to_string())?;
        reply_rx
            .recv()
            .map_err(|_| "Audio worker dropped reply".to_string())??;
        Ok(())
    }

    pub fn stop_and_drain(&mut self) -> Result<DrainedAudio, String> {
        let (reply_tx, reply_rx) = channel();
        self.cmd_tx
            .send(AudioCommand::Stop { reply: reply_tx })
            .map_err(|_| "Audio worker thread is not running".to_string())?;
        reply_rx
            .recv()
            .map_err(|_| "Audio worker dropped reply".to_string())?
    }

    pub fn stop_and_save(&mut self, output_path: &Path) -> Result<f32, String> {
        let drained = self.stop_and_drain()?;

        if drained.samples.is_empty() {
            return Err(
                "No audio captured. Check that your microphone is enabled and not muted.".into(),
            );
        }

        let mono = downmix_mono(&drained.samples, drained.channels);
        let duration_secs = mono.len() as f32 / drained.sample_rate as f32;
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
            return Err(format!(
                "Microphone captured silence (peak {:.4}). \
                 Check Windows mic permissions, mute switch, and that the right input is selected in Settings.",
                peak
            ));
        }

        let resampled = resample(&mono, drained.sample_rate, 16000);
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

        debug!("WAV saved to {}", output_path.display());
        Ok(duration_secs)
    }
}

fn worker_loop(rx: Receiver<AudioCommand>) {
    let mut active: Option<ActiveStream> = None;
    while let Ok(cmd) = rx.recv() {
        match cmd {
            AudioCommand::Start { mic_name, reply } => {
                if active.is_some() {
                    let _ = reply.send(Err("Audio stream already active".to_string()));
                    continue;
                }
                match build_stream(&mic_name) {
                    Ok(a) => {
                        active = Some(a);
                        debug!("Audio recording started");
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            AudioCommand::Stop { reply } => {
                let Some(a) = active.take() else {
                    let _ = reply.send(Err("Not recording".to_string()));
                    continue;
                };
                let samples = std::mem::take(&mut *a.samples.lock());
                let sample_rate = a.sample_rate;
                let channels = a.channels;
                let sample_format = a.sample_format;
                drop(a);
                debug!("Audio recording stopped");
                let _ = reply.send(Ok(DrainedAudio {
                    samples,
                    sample_rate,
                    channels,
                    sample_format,
                }));
            }
            AudioCommand::Shutdown => break,
        }
    }
}

fn build_stream(mic_name: &str) -> Result<ActiveStream, String> {
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

    let max_samples = sample_rate as usize * channels as usize * MAX_RECORDING_SECS as usize;
    let config: cpal::StreamConfig = supported.config();
    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let samples_for_cb = samples.clone();
    let err_fn = |e: cpal::StreamError| error!("Audio stream error: {}", e);

    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buf = samples_for_cb.lock();
                if buf.len() >= max_samples { return; }
                let room = max_samples - buf.len();
                if data.len() <= room {
                    buf.extend_from_slice(data);
                } else {
                    buf.extend(data[..room].iter().copied());
                }
            },
            err_fn,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                let mut buf = samples_for_cb.lock();
                if buf.len() >= max_samples { return; }
                let room = max_samples - buf.len();
                buf.extend(data.iter().take(room).map(|&s| s.to_sample::<f32>()));
            },
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                let mut buf = samples_for_cb.lock();
                if buf.len() >= max_samples { return; }
                let room = max_samples - buf.len();
                buf.extend(data.iter().take(room).map(|&s| s.to_sample::<f32>()));
            },
            err_fn,
            None,
        ),
        SampleFormat::I32 => device.build_input_stream(
            &config,
            move |data: &[i32], _: &cpal::InputCallbackInfo| {
                let mut buf = samples_for_cb.lock();
                if buf.len() >= max_samples { return; }
                let room = max_samples - buf.len();
                buf.extend(data.iter().take(room).map(|&s| s.to_sample::<f32>()));
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

    Ok(ActiveStream {
        _stream: stream,
        samples,
        sample_rate,
        channels,
        sample_format,
    })
}

fn downmix_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels > 1 {
        samples
            .chunks(channels as usize)
            .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
            .collect()
    } else {
        samples.to_vec()
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

pub fn test_microphone(mic_name: &str, duration_ms: u32) -> Result<MicTestResult, String> {
    let mut recorder = AudioRecorder::new();
    recorder.start(mic_name)?;
    std::thread::sleep(std::time::Duration::from_millis(duration_ms as u64));
    let drained = recorder.stop_and_drain()?;
    drop(recorder);

    let device_label = if mic_name == "default" {
        cpal::default_host()
            .default_input_device()
            .and_then(|d| d.name().ok())
            .unwrap_or_else(|| "default".into())
    } else {
        mic_name.to_string()
    };
    let format = format!("{:?}", drained.sample_format);

    let mono = downmix_mono(&drained.samples, drained.channels);
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
        sample_rate: drained.sample_rate,
        channels: drained.channels,
        format,
        duration_ms,
        samples_collected: mono.len(),
        peak,
        peak_db,
        rms,
        verdict,
    })
}

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
