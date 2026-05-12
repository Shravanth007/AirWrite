use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use log::warn;
use std::time::Duration;

const PRE_PASTE_DELAY: Duration = Duration::from_millis(50);

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

    std::thread::sleep(PRE_PASTE_DELAY);

    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init failed: {}", e))?;
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
