use enigo::{Direction, Enigo, Key, Keyboard, Settings};

/// Set the system clipboard to `text` and synthesize Ctrl+V into the foreground window.
pub fn paste_text(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard open failed: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Clipboard write failed: {}", e))?;

    // Brief delay so the clipboard owner change propagates before paste fires.
    std::thread::sleep(std::time::Duration::from_millis(50));

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init failed: {}", e))?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| format!("Ctrl down failed: {}", e))?;
    let click_result = enigo.key(Key::Unicode('v'), Direction::Click);
    // Always release Ctrl, even if the click failed, so we don't leave the
    // modifier stuck.
    let release_result = enigo.key(Key::Control, Direction::Release);

    click_result.map_err(|e| format!("V keystroke failed: {}", e))?;
    release_result.map_err(|e| format!("Ctrl up failed: {}", e))?;
    Ok(())
}
