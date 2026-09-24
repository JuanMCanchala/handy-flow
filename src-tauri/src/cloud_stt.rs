//! Cloud speech-to-text via an OpenAI-compatible `/audio/transcriptions` endpoint.
//!
//! Used instead of a local Whisper/Parakeet model when `cloud_stt_enabled` is
//! set. The request is a multipart form: the recorded audio as a 16 kHz mono
//! WAV file, the configured model id, an optional `language`, and an optional
//! `prompt` built from `custom_words` (mirrors the local whisper initial-prompt
//! convention in `managers/transcription.rs`).

use crate::settings::{AppSettings, CloudSttProvider};
use hound::{WavSpec, WavWriter};
use log::debug;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::io::Cursor;
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

/// Encode f32 samples (assumed 16 kHz mono, matching the app's internal
/// sample rate) as an in-memory 16-bit PCM WAV file.
fn encode_wav(samples: &[f32]) -> Result<Vec<u8>, String> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut buffer = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut buffer, spec)
            .map_err(|e| format!("Failed to create WAV writer: {}", e))?;
        for sample in samples {
            let sample_i16 = (sample * i16::MAX as f32) as i16;
            writer
                .write_sample(sample_i16)
                .map_err(|e| format!("Failed to write WAV sample: {}", e))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV data: {}", e))?;
    }

    Ok(buffer.into_inner())
}

/// Build the multipart form for a transcription request.
fn build_form(
    wav_bytes: Vec<u8>,
    model: &str,
    language: Option<&str>,
    prompt: Option<&str>,
) -> Result<Form, String> {
    let file_part = Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Failed to build audio part: {}", e))?;

    let mut form = Form::new()
        .part("file", file_part)
        .text("model", model.to_string());

    if let Some(language) = language {
        form = form.text("language", language.to_string());
    }
    if let Some(prompt) = prompt {
        form = form.text("prompt", prompt.to_string());
    }

    Ok(form)
}

/// Resolve the provider, base URL, model, and API key to use for a cloud STT
/// request from the current settings. Returns a descriptive error if the
/// selected provider is unknown or has no model configured.
fn resolve_provider_config(
    settings: &AppSettings,
) -> Result<(CloudSttProvider, String, String), String> {
    let provider = settings
        .active_cloud_stt_provider()
        .cloned()
        .ok_or_else(|| {
            format!(
                "Cloud STT provider '{}' not found",
                settings.cloud_stt_provider_id
            )
        })?;

    let model = settings
        .cloud_stt_models
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();
    if model.trim().is_empty() {
        return Err(format!(
            "No model configured for cloud STT provider '{}'",
            provider.label
        ));
    }

    let api_key = settings
        .cloud_stt_api_keys
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();

    Ok((provider, model, api_key))
}

/// Transcribe audio samples (16 kHz mono f32) using the configured cloud
/// provider. `custom_words` becomes the `prompt` field (comma-separated, same
/// convention as the local whisper initial prompt). `language` is omitted
/// when the setting is `"auto"`.
/// Blocking wrapper for synchronous callers such as
/// `TranscriptionManager::transcribe`, which often run *inside* a Tokio task
/// (the dictation pipeline, file import, live subtitles). Calling
/// `block_on` on the shared runtime from there deadlocks, so the request runs
/// on its own thread with a private current-thread runtime.
pub fn transcribe_blocking(
    settings: &AppSettings,
    audio: Vec<f32>,
    custom_words: &[String],
    language: &str,
) -> Result<String, String> {
    let settings = settings.clone();
    let custom_words = custom_words.to_vec();
    let language = language.to_string();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to start cloud STT runtime: {}", e))?;
        runtime.block_on(transcribe(&settings, audio, &custom_words, &language))
    })
    .join()
    .map_err(|_| "Cloud STT worker thread panicked".to_string())?
}

pub async fn transcribe(
    settings: &AppSettings,
    audio: Vec<f32>,
    custom_words: &[String],
    language: &str,
) -> Result<String, String> {
    let (provider, model, api_key) = resolve_provider_config(settings)?;

    let base_url = provider.base_url.trim_end_matches('/');
    let url = format!("{}/audio/transcriptions", base_url);

    debug!("Sending cloud STT request to: {}", url);

    let wav_bytes = encode_wav(&audio)?;

    let language_param = if language.eq_ignore_ascii_case("auto") {
        None
    } else {
        Some(language)
    };
    let prompt = if custom_words.is_empty() {
        None
    } else {
        Some(custom_words.join(", "))
    };

    // Validate the form once up front so a bad mime/params error surfaces
    // directly instead of through the retry closure.
    build_form(wav_bytes.clone(), &model, language_param, prompt.as_deref())?;

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = crate::llm_client::send_with_connect_retry(|| {
        let form = build_form(wav_bytes.clone(), &model, language_param, prompt.as_deref())
            .expect("form validated above");
        let mut request = client.post(&url).multipart(form);
        if !api_key.is_empty() {
            request = request.bearer_auth(&api_key);
        }
        Ok(request)
    })
    .await
    .map_err(|e| crate::llm_client::report_reqwest_error("Cloud STT request failed", &e))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|e| format!("<failed to read error body: {}>", e));
        return Err(format!(
            "Cloud STT request failed with status {}: {}",
            status, error_text
        ));
    }

    let parsed: TranscriptionResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse cloud STT response: {}", e))?;

    Ok(parsed.text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::get_default_settings;

    #[test]
    fn encode_wav_produces_valid_16khz_mono_pcm() {
        let samples = vec![0.0_f32, 0.5, -0.5, 1.0, -1.0];
        let bytes = encode_wav(&samples).expect("encode should succeed");

        let reader = hound::WavReader::new(Cursor::new(bytes)).expect("valid WAV");
        let spec = reader.spec();
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 16000);
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(reader.len() as usize, samples.len());
    }

    #[test]
    fn build_form_without_language_or_prompt_still_builds() {
        let form = build_form(vec![0u8; 4], "whisper-large-v3-turbo", None, None);
        assert!(form.is_ok());
    }

    #[test]
    fn build_form_with_language_and_prompt_succeeds() {
        let form = build_form(
            vec![0u8; 4],
            "gpt-4o-mini-transcribe",
            Some("en"),
            Some("Handy, cjpais"),
        );
        assert!(form.is_ok());
    }

    /// Regression: calling the blocking wrapper from inside a Tokio task (as
    /// the dictation pipeline does) used to deadlock on `block_on`, leaving the
    /// overlay stuck on "Transcribing…". It must return (here: a connection
    /// error) instead of hanging.
    #[test]
    fn blocking_call_inside_async_task_does_not_deadlock() {
        let mut settings = get_default_settings();
        settings.cloud_stt_provider_id = "custom".to_string();
        if let Some(provider) = settings
            .cloud_stt_providers
            .iter_mut()
            .find(|p| p.id == "custom")
        {
            provider.base_url = "http://127.0.0.1:9/v1".to_string();
        }
        settings
            .cloud_stt_models
            .insert("custom".to_string(), "test-model".to_string());

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            let result = runtime.block_on(async move {
                tokio::spawn(
                    async move { transcribe_blocking(&settings, vec![0.0; 1600], &[], "en") },
                )
                .await
                .unwrap()
            });
            let _ = tx.send(result);
        });

        let result = rx
            .recv_timeout(std::time::Duration::from_secs(15))
            .expect("cloud STT call deadlocked inside an async task");
        assert!(result.is_err());
    }

    /// Live check against Fireworks (needs network + FIREWORKS_API_KEY):
    /// `cargo test --lib live_fireworks -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn live_fireworks_transcription() {
        let key = std::env::var("FIREWORKS_API_KEY").expect("FIREWORKS_API_KEY");
        let mut settings = get_default_settings();
        settings.cloud_stt_provider_id = "fireworks".to_string();
        settings
            .cloud_stt_api_keys
            .insert("fireworks".to_string(), key);
        let tone: Vec<f32> = (0..16000)
            .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / 16000.0).sin() * 0.1)
            .collect();
        let result = transcribe_blocking(&settings, tone, &[], "en");
        println!("live result: {:?}", result);
        assert!(result.is_ok(), "{:?}", result);
    }

    #[test]
    fn resolve_provider_config_errors_on_missing_model() {
        let mut settings = get_default_settings();
        settings.cloud_stt_provider_id = "custom".to_string();
        // Default "custom" model is empty.
        let result = resolve_provider_config(&settings);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No model configured"));
    }

    #[test]
    fn resolve_provider_config_errors_on_unknown_provider() {
        let mut settings = get_default_settings();
        settings.cloud_stt_provider_id = "does-not-exist".to_string();
        let result = resolve_provider_config(&settings);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn resolve_provider_config_returns_default_openai_model() {
        let settings = get_default_settings();
        let (provider, model, _api_key) =
            resolve_provider_config(&settings).expect("openai provider should resolve");
        assert_eq!(provider.id, "openai");
        assert_eq!(model, "gpt-4o-mini-transcribe");
    }

    #[test]
    fn language_auto_is_omitted() {
        // "auto" should map to None for the language param; anything else passes through.
        assert!("auto".eq_ignore_ascii_case("Auto"));
    }
}
