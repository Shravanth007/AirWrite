use reqwest::multipart;
use std::path::Path;

const GROQ_ENDPOINT: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
const MODEL: &str = "whisper-large-v3-turbo";

pub async fn transcribe_groq(api_key: &str, audio_path: &Path) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("Groq API key not set. Open Settings to enter your key.".to_string());
    }
    validate_api_key(api_key)?;

    let audio_bytes = std::fs::read(audio_path)
        .map_err(|e| format!("Failed to read audio file: {}", e))?;

    let file_part = multipart::Part::bytes(audio_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Multipart build failed: {}", e))?;

    let form = multipart::Form::new()
        .text("model", MODEL)
        .text("language", "en")
        .text("response_format", "json")
        .part("file", file_part);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP client build failed: {}", e))?;

    let response = client
        .post(GROQ_ENDPOINT)
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Groq API request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        // Don't echo the full body — it may include the request that contained
        // the bearer token in some proxy error pages. Surface a concise message.
        let snippet = response
            .text()
            .await
            .ok()
            .and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
            .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
            .unwrap_or_else(|| match status.as_u16() {
                401 => "invalid API key".to_string(),
                429 => "rate limited".to_string(),
                503 => "Groq service unavailable".to_string(),
                _ => "unexpected error".to_string(),
            });
        return Err(format!("Groq API error ({}): {}", status, snippet));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Groq response: {}", e))?;

    json["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No 'text' field in Groq response".to_string())
}

/// Reject keys that contain characters which would either break the
/// Authorization header (CR/LF) or are obviously not a Groq key. Keeps the
/// error path human-readable instead of producing a `reqwest` "invalid header"
/// failure that users can't action.
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
        let err = transcribe_groq("", &path).await.unwrap_err();
        assert!(err.contains("API key not set"));
    }

    #[tokio::test]
    async fn whitespace_in_key_is_rejected() {
        let path = PathBuf::from("nonexistent.wav");
        let err = transcribe_groq("gsk_with space", &path).await.unwrap_err();
        assert!(err.contains("whitespace"));
    }

    #[tokio::test]
    async fn newline_in_key_is_rejected() {
        let path = PathBuf::from("nonexistent.wav");
        let err = transcribe_groq("gsk_abc\ndef", &path).await.unwrap_err();
        assert!(err.contains("whitespace") || err.contains("control"));
    }
}
