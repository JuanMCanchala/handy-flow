//! Command mode: a spoken instruction is applied by the LLM to the text the
//! user has selected (or, with nothing selected, answered as new text), and the
//! result replaces the selection in place — Wispr Flow's "Command Mode".
//!
//! Entered either through the dedicated `command` shortcut, or by addressing
//! the agent by name at the start of a normal dictation ("Hey Flow, ...").

use crate::input::{self, EnigoState};
use crate::settings::AppSettings;
use log::{debug, warn};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// How long to wait for the focused app to answer the synthetic copy.
const COPY_TIMEOUT: Duration = Duration::from_millis(400);
const COPY_POLL: Duration = Duration::from_millis(20);

/// Greetings that may precede the agent name, in the languages users dictate most.
const GREETINGS: &[&str] = &["hey", "hi", "ok", "okay", "oye", "hola", "ey", "eh"];

/// If `transcription` opens by addressing the agent ("Hey Flow, make this
/// formal"), returns the instruction that follows the address. Matching is
/// case-, accent- and punctuation-insensitive so ASR variations still route.
pub fn strip_agent_address(transcription: &str, agent_name: &str) -> Option<String> {
    let agent = normalize_word(agent_name.trim());
    if agent.is_empty() {
        return None;
    }

    let words: Vec<&str> = transcription.split_whitespace().collect();
    let mut idx = 0;
    if let Some(first) = words.first() {
        if GREETINGS.contains(&normalize_word(first).as_str()) {
            idx = 1;
        }
    }

    // Agent names may be several words ("Jarvis Two"); compare word by word.
    let agent_words: Vec<String> = agent.split(' ').map(str::to_string).collect();
    if words.len() < idx + agent_words.len() {
        return None;
    }
    for (offset, expected) in agent_words.iter().enumerate() {
        if &normalize_word(words[idx + offset]) != expected {
            return None;
        }
    }

    let rest = words[idx + agent_words.len()..].join(" ");
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace());
    if rest.is_empty() {
        return None;
    }
    Some(rest.to_string())
}

fn normalize_word(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .map(fold_accent)
        .collect::<String>()
        .to_lowercase()
}

fn fold_accent(c: char) -> char {
    match c {
        'á' | 'à' | 'ä' | 'â' | 'Á' | 'À' | 'Ä' | 'Â' => 'a',
        'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' | 'Ó' | 'Ò' | 'Ö' | 'Ô' => 'o',
        'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => 'u',
        'ñ' | 'Ñ' => 'n',
        other => other,
    }
}

/// System prompt for command mode. The selection and instruction travel in the
/// user message so the prompt stays cacheable across calls.
pub const COMMAND_SYSTEM_PROMPT: &str = "You are a writing assistant embedded in a dictation app. \
The user speaks an instruction. If SELECTED TEXT is provided, apply the instruction to it and \
return only the rewritten text. If there is no selected text, write the text the instruction asks \
for. Output only the final text that should be inserted: no explanations, no preamble, no quotes, \
no markdown fences. Keep the language of the selected text unless the instruction asks for a \
translation.";

pub fn build_command_user_message(selection: Option<&str>, instruction: &str) -> String {
    match selection.map(str::trim).filter(|s| !s.is_empty()) {
        Some(selection) => format!(
            "SELECTED TEXT:\n<<<\n{}\n>>>\n\nINSTRUCTION:\n{}",
            selection,
            instruction.trim()
        ),
        None => format!("INSTRUCTION:\n{}", instruction.trim()),
    }
}

/// Copies the current selection of the focused app via a synthetic copy and
/// returns it, restoring the user's clipboard afterwards. Returns `None` when
/// nothing is selected (the copy leaves the clipboard empty).
pub fn capture_selection(app: &AppHandle) -> Option<String> {
    let clipboard = app.clipboard();
    let saved = clipboard.read_text().ok().filter(|t| !t.is_empty());

    // Empty the clipboard so an unchanged clipboard can't be mistaken for a
    // selection when the app has nothing selected.
    if let Err(e) = clipboard.clear() {
        warn!("command mode: could not clear clipboard: {}", e);
        return None;
    }

    let copied = {
        let state = app.try_state::<EnigoState>()?;
        let mut enigo = state.0.lock().ok()?;
        input::send_copy_ctrl_c(&mut enigo, 60)
    };

    let mut selection = None;
    if let Err(e) = copied {
        warn!("command mode: synthetic copy failed: {}", e);
    } else {
        let started = Instant::now();
        while started.elapsed() < COPY_TIMEOUT {
            std::thread::sleep(COPY_POLL);
            if let Ok(text) = clipboard.read_text() {
                if !text.is_empty() {
                    selection = Some(text);
                    break;
                }
            }
        }
    }

    match saved {
        Some(text) => {
            let _ = clipboard.write_text(text);
        }
        None => {
            let _ = clipboard.clear();
        }
    }

    debug!(
        "command mode: selection captured = {}",
        selection.as_ref().map(|s| s.len()).unwrap_or(0)
    );
    selection
}

/// Runs the instruction against the selection with the configured LLM
/// provider (the same one post-processing uses).
pub async fn run_command(
    settings: &AppSettings,
    selection: Option<&str>,
    instruction: &str,
) -> Result<String, String> {
    let (provider, model, api_key) = settings
        .resolve_llm_target()
        .ok_or_else(|| "No AI provider/model configured for command mode".to_string())?;

    let content = crate::llm_client::send_chat_completion_with_schema(
        &provider,
        api_key,
        &model,
        build_command_user_message(selection, instruction),
        Some(COMMAND_SYSTEM_PROMPT.to_string()),
        None,
        crate::llm_client::should_disable_reasoning(&provider),
    )
    .await?
    .ok_or_else(|| "The AI provider returned an empty response".to_string())?;

    let content =
        crate::actions::strip_invisible_chars(crate::actions::strip_think_block(&content));
    let content = strip_code_fence(content.trim());
    if content.is_empty() {
        return Err("The AI provider returned an empty response".to_string());
    }
    Ok(content.to_string())
}

fn strip_code_fence(s: &str) -> &str {
    let Some(rest) = s.strip_prefix("```") else {
        return s;
    };
    let rest = rest.split_once('\n').map(|(_, body)| body).unwrap_or("");
    rest.strip_suffix("```").unwrap_or(rest).trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_with_greeting_is_stripped() {
        assert_eq!(
            strip_agent_address("Hey Flow, make this more formal.", "Flow").as_deref(),
            Some("make this more formal.")
        );
    }

    #[test]
    fn address_without_greeting_and_spanish_greeting() {
        assert_eq!(
            strip_agent_address("flow traduce esto al inglés", "Flow").as_deref(),
            Some("traduce esto al inglés")
        );
        assert_eq!(
            strip_agent_address("Oye, Flow: resume esto", "flow").as_deref(),
            Some("resume esto")
        );
    }

    #[test]
    fn multi_word_and_accented_agent_names() {
        assert_eq!(
            strip_agent_address("hey Jarvis Two summarize", "Jarvis two").as_deref(),
            Some("summarize")
        );
        assert_eq!(
            strip_agent_address("Hola Martín, corrige", "Martin").as_deref(),
            Some("corrige")
        );
    }

    #[test]
    fn plain_dictation_is_not_a_command() {
        assert_eq!(
            strip_agent_address("I went with the flow today", "Flow"),
            None
        );
        assert_eq!(strip_agent_address("Hey Flow", "Flow"), None);
        assert_eq!(strip_agent_address("hey there", "Flow"), None);
        assert_eq!(strip_agent_address("anything", ""), None);
    }

    #[test]
    fn user_message_includes_selection_only_when_present() {
        let with = build_command_user_message(Some(" hola "), "translate");
        assert!(with.contains("SELECTED TEXT") && with.contains("hola"));
        let without = build_command_user_message(Some("  "), "write a haiku");
        assert_eq!(without, "INSTRUCTION:\nwrite a haiku");
    }

    #[test]
    fn code_fences_are_removed() {
        assert_eq!(strip_code_fence("```text\nhello\n```"), "hello");
        assert_eq!(strip_code_fence("hello"), "hello");
    }
}
