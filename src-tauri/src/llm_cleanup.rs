//! Optional post-Whisper polish via Groq's Llama 3.3 70B Versatile.
//!
//! The cleanup pass fixes grammar, punctuation, and capitalisation, and
//! drops disfluencies ("um", "uh", "like") that Whisper transcribes
//! verbatim. It is opt-in (`Settings::ai_cleanup_enabled`) because it adds
//! 0.3–1.0s of round-trip latency.
//!
//! Failure of this pass is **never fatal**: the caller falls back to the
//! raw Whisper output so the user never loses a dictation just because the
//! polish step had a bad day.

use log::warn;
use serde_json::json;
use std::time::Duration;

const GROQ_CHAT_ENDPOINT: &str = "https://api.groq.com/openai/v1/chat/completions";
const CLEANUP_MODEL: &str = "llama-3.3-70b-versatile";

/// Hard ceiling on a single cleanup call. Whisper's own request has a 60s
/// timeout; 15s here keeps a flaky LLM from holding the paste back too long.
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(15);

const SYSTEM_PROMPT: &str = "You are a transcription editor. The user just spoke and Whisper transcribed it. \
Your job: clean up the transcription by fixing grammar, punctuation, capitalisation, \
and removing filler words ('um', 'uh', 'like', 'you know') and obvious self-corrections. \
PRESERVE the user's exact meaning, voice, vocabulary, and tone. Do NOT translate, \
summarise, expand, rephrase, or change wording beyond what is necessary for grammatical \
correctness. Do NOT add commentary, preambles, quotes, or any text other than the \
cleaned transcription. If the input is already clean, return it unchanged.";

/// Polish `raw_text` via Groq's chat completions endpoint. On any failure
/// (network, HTTP, parse, empty response) returns `Err` — the caller is
/// expected to fall back to `raw_text` so the user's dictation is never lost.
pub async fn cleanup_with_llm(api_key: &str, raw_text: &str) -> Result<String, String> {
    let trimmed = raw_text.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if api_key.trim().is_empty() {
        return Err("Groq API key not set".to_string());
    }

    let body = json!({
        "model": CLEANUP_MODEL,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": trimmed}
        ],
        "temperature": 0.2,
        "max_tokens": 2048,
    });

    let client = reqwest::Client::builder()
        .timeout(CLEANUP_TIMEOUT)
        .build()
        .map_err(|e| format!("HTTP client build failed: {}", e))?;

    let response = client
        .post(GROQ_CHAT_ENDPOINT)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            warn!("LLM cleanup network error: {}", e);
            format!("LLM cleanup request failed: {}", e)
        })?;

    let status = response.status();
    if !status.is_success() {
        // Same caution as the Whisper path — never echo the raw body, since
        // some proxy errors include the request payload.
        return Err(format!("LLM cleanup HTTP {}", status));
    }

    let json_resp: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

    let content = json_resp["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "No content in LLM response".to_string())?;

    let cleaned = content.trim();
    if cleaned.is_empty() {
        // Don't replace good text with empty text — bubble up so caller falls
        // back to the raw transcription.
        return Err("LLM returned empty content".to_string());
    }
    Ok(cleaned.to_string())
}
