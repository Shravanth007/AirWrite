use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use log::warn;
use std::time::Duration;

/// Delay between setting the clipboard and synthesising Ctrl+V. Gives the
/// clipboard owner change a beat to propagate before the focused app reads it.
const PRE_PASTE_DELAY: Duration = Duration::from_millis(50);

/// Delay before we restore the prior clipboard, after the paste keystroke
/// fires. Long enough for the destination app's WM_PASTE handler to read
/// the clipboard, short enough that the user doesn't notice their old
/// clipboard "missing" if they hit Ctrl+V manually right after dictation.
const RESTORE_DELAY: Duration = Duration::from_millis(500);

/// Set the system clipboard to `text` and synthesise Ctrl+V into the
/// foreground window. If `restore_clipboard` is true, snapshot whatever
/// text was on the clipboard first and restore it ~500ms after the paste —
/// but only if the clipboard still contains exactly what we just pasted
/// (i.e. nobody copied something new in the meantime).
pub fn paste_text(text: &str, restore_clipboard: bool) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Clipboard open failed: {}", e))?;

    // Snapshot the prior clipboard text BEFORE we clobber it. We only
    // restore text — image/file restoration is out of scope (and `arboard`
    // makes them awkward). If the prior clipboard had a non-text payload,
    // the snapshot is `None` and we just skip the restore.
    let prior_text = if restore_clipboard {
        clipboard.get_text().ok()
    } else {
        None
    };

    clipboard
        .set_text(text)
        .map_err(|e| format!("Clipboard write failed: {}", e))?;

    std::thread::sleep(PRE_PASTE_DELAY);

    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init failed: {}", e))?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| format!("Ctrl down failed: {}", e))?;
    let click_result = enigo.key(Key::Unicode('v'), Direction::Click);
    // Always release Ctrl, even if the click failed, so we don't leave the
    // modifier stuck.
    let release_result = enigo.key(Key::Control, Direction::Release);

    click_result.map_err(|e| format!("V keystroke failed: {}", e))?;
    release_result.map_err(|e| format!("Ctrl up failed: {}", e))?;

    // Schedule a restore on a background thread. We compare the current
    // clipboard against `text` before restoring — if the user (or another
    // dictation) copied something new, the clipboard won't match and we
    // skip, so we don't clobber the new content.
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
                    // Something new is on the clipboard — leave it alone.
                }
                Err(e) => {
                    warn!("Clipboard restore: get failed: {}", e);
                }
            }
        });
    }

    Ok(())
}
