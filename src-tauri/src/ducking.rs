//! Audio ducking — temporarily lowers the system master output volume while
//! a recording is in progress, so playing music/video doesn't bleed into the
//! mic. Restores the prior level when recording stops.
//!
//! Uses the Windows Core Audio API (`IAudioEndpointVolume`) directly via the
//! `windows` crate. Failure here is always non-fatal — the recording is more
//! important than the duck.
//!
//! Crash-recovery: every time we duck, we drop the prior volume level into a
//! tiny `pre_duck.txt` next to `config.json`. On clean stop we delete it.
//! On startup the app reads the file (if present) and restores from it before
//! anything else, so a Task-Manager kill / BSOD / panic mid-recording doesn't
//! leave the user's master volume stuck at 15%.

use log::{info, warn};
use std::fs;
use std::path::Path;
use windows::core::Result as WinResult;
use windows::Win32::Media::Audio::{
    eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};

/// Initialise COM for this thread. Idempotent — `S_FALSE` (already
/// initialised) and `RPC_E_CHANGED_MODE` (different apartment already in
/// use on this thread) are both treated as success: in either case we can
/// still make COM calls from here. Failure on first init is propagated.
fn ensure_com() -> WinResult<()> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        if hr.is_ok() || hr.0 == 1 /* S_FALSE */ || hr.0 == 0x80010106u32 as i32 /* RPC_E_CHANGED_MODE */ {
            Ok(())
        } else {
            Err(windows::core::Error::from_hresult(hr))
        }
    }
}

fn get_endpoint_volume() -> WinResult<IAudioEndpointVolume> {
    ensure_com()?;
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_INPROC_SERVER, None)?;
        Ok(volume)
    }
}

/// Read the current master output volume (0.0–1.0) and set it to
/// `target_percent`% of the maximum (i.e. 15 → 0.15). Returns the prior
/// value so the caller can restore it later.
pub fn duck(target_percent: u8) -> Result<f32, String> {
    let volume = get_endpoint_volume().map_err(|e| format!("Audio endpoint init failed: {}", e))?;
    let target = (target_percent.min(100) as f32) / 100.0;
    unsafe {
        let prior = volume
            .GetMasterVolumeLevelScalar()
            .map_err(|e| format!("Read master volume failed: {}", e))?;
        // Don't push the volume *up* if it was already lower than the target.
        // (User had it set to 5%, we'd rather not bump it to 15%.)
        if prior <= target {
            return Ok(prior);
        }
        volume
            .SetMasterVolumeLevelScalar(target, std::ptr::null())
            .map_err(|e| format!("Set master volume failed: {}", e))?;
        Ok(prior)
    }
}

/// Restore the master output volume to `level` (0.0–1.0). Best-effort —
/// failures are logged but never propagated, since we never want a
/// post-recording cleanup error to surface to the user.
pub fn restore(level: f32) {
    let level = level.clamp(0.0, 1.0);
    let volume = match get_endpoint_volume() {
        Ok(v) => v,
        Err(e) => {
            warn!("Audio restore: endpoint init failed: {}", e);
            return;
        }
    };
    unsafe {
        if let Err(e) = volume.SetMasterVolumeLevelScalar(level, std::ptr::null()) {
            warn!("Audio restore: set volume failed: {}", e);
        }
    }
}

/// Persist the pre-duck level so we can recover if the process dies before
/// `restore` is called. Stored as an integer percentage 0–100 (one line of
/// ASCII) — the format is intentionally trivial so a partial write is easy
/// to spot. Best-effort: a write failure does not block recording.
pub fn save_pending(level: f32, path: &Path) {
    let pct = (level.clamp(0.0, 1.0) * 100.0).round() as u8;
    if let Err(e) = fs::write(path, pct.to_string()) {
        warn!(
            "Could not write duck recovery file {}: {}",
            path.display(),
            e
        );
    }
}

/// Delete the recovery file. Idempotent — a missing file is fine.
pub fn clear_pending(path: &Path) {
    if let Err(e) = fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            warn!(
                "Could not delete duck recovery file {}: {}",
                path.display(),
                e
            );
        }
    }
}

/// Startup hook: if a recovery file is present, restore the master volume
/// from it and remove the file. Garbage / unreadable contents are logged
/// and the file is removed anyway — leaving a corrupt file in place would
/// keep prompting recovery forever.
pub fn restore_pending(path: &Path) {
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            warn!(
                "Could not read duck recovery file {}: {}",
                path.display(),
                e
            );
            return;
        }
    };
    match contents.trim().parse::<u8>() {
        Ok(pct) if pct <= 100 => {
            let level = pct as f32 / 100.0;
            info!(
                "Recovering master volume from prior unclean exit: → {}%",
                pct
            );
            restore(level);
        }
        Ok(pct) => warn!("Duck recovery file had invalid percentage {}, ignoring.", pct),
        Err(e) => warn!(
            "Garbage in duck recovery file {}: {}",
            path.display(),
            e
        ),
    }
    let _ = fs::remove_file(path);
}
