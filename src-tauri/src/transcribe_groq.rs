use reqwest::multipart;
use std::path::Path;

const GROQ_ENDPOINT: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
const MODEL: &str = "whisper-large-v3-turbo";

// ponytail: single retry (2 attempts total) with fixed 500ms backoff.
// Upgrade path: increase MAX_ATTEMPTS and add jittered exponential backoff
// if transient failures remain a reliability concern.
const MAX_ATTEMPTS: u32 = 2;
const RETRY_BACKOFF_MS: u64 = 500;

pub async fn transcribe_groq(
    api_key: &str,
    audio_path: &Path,
    language: &str,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("Groq API key not set. Open Settings to enter your key.".to_string());
    }
    validate_api_key(api_key)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP client build failed: {}", e))?;

    let mut last_err = String::new();

    for attempt in 0..MAX_ATTEMPTS {
        // Rebuild form each attempt: multipart::Form is not Clone.
        let audio_bytes = std::fs::read(audio_path)
            .map_err(|e| format!("Failed to read audio file: {}", e))?;

        let file_part = multipart::Part::bytes(audio_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| format!("Multipart build failed: {}", e))?;

        let mut form = multipart::Form::new()
            .text("model", MODEL)
            .text("response_format", "json");
        // "auto" (or empty) omits the language field so Groq Whisper detects it.
        if !language.is_empty() && language != "auto" {
            form = form.text("language", language.to_string());
        }
        let form = form.part("file", file_part);

        let send_result = client
            .post(GROQ_ENDPOINT)
            .bearer_auth(api_key)
            .multipart(form)
            .send()
            .await;

        match send_result {
            Err(e) => {
                let is_transient = e.is_timeout() || e.is_connect() || e.is_request();
                let msg = classify_request_error(e);
                if is_transient && attempt + 1 < MAX_ATTEMPTS {
                    log::warn!("Transient network error on attempt {}; retrying: {}", attempt + 1, msg);
                    tokio::time::sleep(std::time::Duration::from_millis(RETRY_BACKOFF_MS)).await;
                    last_err = msg;
                    continue;
                }
                return Err(msg);
            }
            Ok(response) => {
                let status = response.status();
                let status_u16 = status.as_u16();

                if !status.is_success() {
                    let is_transient = matches!(status_u16, 408 | 429 | 500..=599);
                    let groq_message = response
                        .text()
                        .await
                        .ok()
                        .and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
                        .and_then(|v| v["error"]["message"].as_str().map(str::to_string));
                    let msg = classify_status_error(status_u16, groq_message);
                    if is_transient && attempt + 1 < MAX_ATTEMPTS {
                        log::warn!("Transient HTTP {} on attempt {}; retrying: {}", status_u16, attempt + 1, msg);
                        tokio::time::sleep(std::time::Duration::from_millis(RETRY_BACKOFF_MS)).await;
                        last_err = msg;
                        continue;
                    }
                    return Err(msg);
                }

                let json: serde_json::Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse Groq response: {}", e))?;

                return json["text"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| "No 'text' field in Groq response".to_string());
            }
        }
    }

    Err(last_err)
}

fn classify_request_error(e: reqwest::Error) -> String {
    let raw = e.to_string();
    log::warn!("Network error during transcribe: {}", raw);

    if e.is_timeout() {
        return "Request timed out. Groq might be slow right now — try again.".to_string();
    }
    if e.is_connect() {
        if raw.contains("dns") || raw.contains("lookup") || raw.contains("resolve") {
            return "Can't reach api.groq.com. You may be offline or behind a firewall.".to_string();
        }
        return "Can't connect to Groq. Check your internet connection.".to_string();
    }
    if e.is_request() {
        return "The request couldn't be built. Try restarting AirWrite.".to_string();
    }
    "Network error while contacting Groq. Try again in a moment.".to_string()
}

fn classify_status_error(status: u16, groq_message: Option<String>) -> String {
    match status {
        401 | 403 => "Your Groq API key was rejected. Open Settings → API key to update it.".to_string(),
        408 => "Groq took too long to respond. Try again.".to_string(),
        413 => "Recording is too long for Groq. Keep dictations under ~25 MB of audio.".to_string(),
        429 => "Too many requests. Wait a few seconds and try again.".to_string(),
        500..=599 => "Groq is having issues right now. Try again in a moment.".to_string(),
        _ => {
            if let Some(m) = groq_message.filter(|m| !m.is_empty() && m.len() < 200) {
                format!("Groq returned {}: {}", status, m)
            } else {
                format!("Unexpected response from Groq ({}). Try again.", status)
            }
        }
    }
}

fn validate_api_key(key: &str) -> Result<(), String> {
    if key.len() > 256 {
        return Err("API key looks too long — copy/paste error?".to_string());
    }
    if key.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("API key contains whitespace or control characters.".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn empty_key_is_rejected() {
        let path = PathBuf::from("nonexistent.wav");
        let err = transcribe_groq("", &path, "en").await.unwrap_err();
        assert!(err.contains("API key not set"));
    }

    #[tokio::test]
    async fn whitespace_in_key_is_rejected() {
        let path = PathBuf::from("nonexistent.wav");
        let err = transcribe_groq("gsk_with space", &path, "en").await.unwrap_err();
        assert!(err.contains("whitespace"));
    }

    #[tokio::test]
    async fn newline_in_key_is_rejected() {
        let path = PathBuf::from("nonexistent.wav");
        let err = transcribe_groq("gsk_abc\ndef", &path, "en").await.unwrap_err();
        assert!(err.contains("whitespace") || err.contains("control"));
    }
}
