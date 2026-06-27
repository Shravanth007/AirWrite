use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use log::warn;
use std::time::{Duration, Instant};

// ponytail: we verify the clipboard write landed; we still cannot verify the target app
// *read* it (that's inherent to clipboard paste — no OS API exposes that).
const CLIPBOARD_POLL_INTERVAL: Duration = Duration::from_millis(10);
const CLIPBOARD_VERIFY_TIMEOUT: Duration = Duration::from_millis(300);

const RESTORE_DELAY: Duration = Duration::from_millis(500);

pub fn paste_text(text: &str, restore_clipboard: bool) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Clipboard open failed: {}", e))?;

    let prior_text = if restore_clipboard {
        clipboard.get_text().ok()
    } else {
        None
    };

    clipboard
        .set_text(text)
        .map_err(|e| format!("Clipboard write failed: {}", e))?;

    // Poll the clipboard back until it matches what we wrote, confirming the write landed
    // before we send Ctrl+V. In the common case this exits in a few ms; 300ms is the safety cap.
    let deadline = Instant::now() + CLIPBOARD_VERIFY_TIMEOUT;
    loop {
        match clipboard.get_text() {
            Ok(ref current) if current == text => break,
            _ => {}
        }
        if Instant::now() >= deadline {
            warn!("paste_text: clipboard verify timed out after {}ms — proceeding best-effort",
                CLIPBOARD_VERIFY_TIMEOUT.as_millis());
            break;
        }
        std::thread::sleep(CLIPBOARD_POLL_INTERVAL);
    }

    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init failed: {}", e))?;
    // ponytail: PTT release can leave Ctrl/Shift/Alt/Win physically held; release them so the
    // synthetic Ctrl+V isn't seen as Ctrl+Shift+V etc. Release on an un-pressed key is a harmless no-op.
    for k in [Key::Shift, Key::Alt, Key::Meta, Key::Control] {
        let _ = enigo.key(k, Direction::Release);
    }
    std::thread::sleep(Duration::from_millis(20));
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| format!("Ctrl down failed: {}", e))?;
    let click_result = enigo.key(Key::Unicode('v'), Direction::Click);
    let release_result = enigo.key(Key::Control, Direction::Release);

    click_result.map_err(|e| format!("V keystroke failed: {}", e))?;
    release_result.map_err(|e| format!("Ctrl up failed: {}", e))?;

    if let Some(prior) = prior_text {
        let pasted_owned = text.to_string();
        std::thread::spawn(move || {
            std::thread::sleep(RESTORE_DELAY);
            let mut cb = match arboard::Clipboard::new() {
                Ok(c) => c,
                Err(e) => {
                    warn!("Clipboard restore: open failed: {}", e);
                    return;
                }
            };
            match cb.get_text() {
                Ok(current) if current == pasted_owned => {
                    if let Err(e) = cb.set_text(&prior) {
                        warn!("Clipboard restore: set failed: {}", e);
                    }
                }
                Ok(_) => {
                }
                Err(e) => {
                    warn!("Clipboard restore: get failed: {}", e);
                }
            }
        });
    }

    Ok(())
}
