
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

fn ensure_com() -> WinResult<()> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        if hr.is_ok() || hr.0 == 1 || hr.0 == 0x80010106u32 as i32 {
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

pub fn duck(target_percent: u8) -> Result<f32, String> {
    let volume = get_endpoint_volume().map_err(|e| format!("Audio endpoint init failed: {}", e))?;
    let target = (target_percent.min(100) as f32) / 100.0;
    unsafe {
        let prior = volume
            .GetMasterVolumeLevelScalar()
            .map_err(|e| format!("Read master volume failed: {}", e))?;
        if prior <= target {
            return Ok(prior);
        }
        volume
            .SetMasterVolumeLevelScalar(target, std::ptr::null())
            .map_err(|e| format!("Set master volume failed: {}", e))?;
        Ok(prior)
    }
}

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
