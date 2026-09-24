//! Named modes (Superwhisper-style presets): user-defined post-processing
//! configurations `{id, name, prompt, provider/model override, language,
//! output format, hotkey}`. Pure resolution logic only — no I/O, no Tauri
//! state. Callers (`actions.rs`) look up the active mode in
//! `AppSettings::modes` and pass it plus the per-app style instruction here.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Output shape a mode's prompt should aim for. Purely descriptive: it is
/// appended to the prompt as an instruction fragment, the same way
/// `style.rs` appends its tone fragment.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModeOutputFormat {
    #[default]
    Plain,
    BulletList,
    Email,
    Markdown,
}

impl ModeOutputFormat {
    /// Instruction fragment appended to the mode's prompt, or `None` for
    /// `Plain` (no extra formatting instruction needed).
    pub fn instruction(&self) -> Option<&'static str> {
        match self {
            ModeOutputFormat::Plain => None,
            ModeOutputFormat::BulletList => {
                Some("Format the output as a concise bullet list.")
            }
            ModeOutputFormat::Email => {
                Some("Format the output as a professional email, with an appropriate greeting and closing if none was dictated.")
            }
            ModeOutputFormat::Markdown => Some("Format the output using Markdown."),
        }
    }
}

/// A user-defined mode/preset.
#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct Mode {
    pub id: String,
    pub name: String,
    /// Post-processing instruction. May contain `${output}` like
    /// `LLMPrompt::prompt`; resolution leaves substitution to the caller.
    pub prompt: String,
    /// Post-processing provider override. `None` means "use the global
    /// post-processing provider/model selection".
    #[serde(default)]
    pub provider_id: Option<String>,
    /// Model override, only meaningful together with `provider_id`.
    #[serde(default)]
    pub model: Option<String>,
    /// Language override. `None` means "use the global selected language".
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub output_format: ModeOutputFormat,
    /// Optional hotkey binding id suffix. When present, the shortcut system
    /// registers a dynamic binding `mode:<id>` for this mode.
    #[serde(default)]
    pub hotkey: Option<String>,
}

/// Prefix for dynamic per-mode shortcut binding ids (`mode:<id>`).
pub const MODE_BINDING_PREFIX: &str = "mode:";

pub fn mode_binding_id(mode_id: &str) -> String {
    format!("{MODE_BINDING_PREFIX}{mode_id}")
}

/// Extracts the mode id from a dynamic binding id (`mode:<id>` -> `<id>`).
pub fn mode_id_from_binding(binding_id: &str) -> Option<&str> {
    binding_id.strip_prefix(MODE_BINDING_PREFIX)
}

pub fn find_mode<'a>(modes: &'a [Mode], id: &str) -> Option<&'a Mode> {
    modes.iter().find(|m| m.id == id)
}

/// What post-processing should actually use once a mode (if any) and the
/// per-app style instruction (if any) are taken into account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMode {
    /// Prompt template, with the mode's output-format instruction and the
    /// style tone fragment appended (mode prompt always wins over the global
    /// prompt when a mode is active).
    pub prompt: String,
    /// Provider id to use, when overridden by the mode.
    pub provider_id: Option<String>,
    /// Model to use, when overridden by the mode (only set together with
    /// `provider_id`).
    pub model: Option<String>,
    /// Language to use, when overridden by the mode.
    pub language: Option<String>,
}

/// Resolve which prompt/model/language post-processing should use.
///
/// - `active_mode`: the currently active [`Mode`], if any (looked up by the
///   caller via `active_mode_id` + `find_mode`).
/// - `global_prompt`: the prompt that would apply with no mode active (the
///   globally selected post-process prompt's template).
/// - `style_instruction`: the per-app tone fragment from `style.rs`, appended
///   on top of whichever prompt wins (mode prompt or global prompt) — style
///   keeps working regardless of mode.
pub fn resolve(
    active_mode: Option<&Mode>,
    global_prompt: &str,
    style_instruction: Option<&str>,
) -> ResolvedMode {
    let Some(mode) = active_mode else {
        return ResolvedMode {
            prompt: append_fragment(global_prompt, style_instruction),
            provider_id: None,
            model: None,
            language: None,
        };
    };

    let mut prompt = mode.prompt.clone();
    if let Some(format_instruction) = mode.output_format.instruction() {
        prompt = append_fragment(&prompt, Some(format_instruction));
    }
    if let Some(language) = mode.language.as_deref().filter(|l| !l.is_empty()) {
        prompt = append_fragment(
            &prompt,
            Some(&format!("Write the output in this language: {language}.")),
        );
    }
    prompt = append_fragment(&prompt, style_instruction);

    ResolvedMode {
        prompt,
        provider_id: mode.provider_id.clone(),
        model: mode.model.clone(),
        language: mode.language.clone(),
    }
}

fn append_fragment(base: &str, fragment: Option<&str>) -> String {
    match fragment {
        Some(fragment) if !fragment.is_empty() => format!("{base}\n\n{fragment}"),
        _ => base.to_string(),
    }
}

/// Seed modes created on fresh installs: Default, Email, Notes.
pub fn default_modes() -> Vec<Mode> {
    vec![
        Mode {
            id: "default_mode_default".to_string(),
            name: "Default".to_string(),
            prompt: "<transcript>\n${output}\n</transcript>\n\nClean up the transcript above: fix spelling, capitalization and punctuation, remove filler words, and keep the original meaning and language.".to_string(),
            provider_id: None,
            model: None,
            language: None,
            output_format: ModeOutputFormat::Plain,
            hotkey: None,
        },
        Mode {
            id: "default_mode_email".to_string(),
            name: "Email".to_string(),
            prompt: "<transcript>\n${output}\n</transcript>\n\nRewrite the transcript above as a clear, professional email.".to_string(),
            provider_id: None,
            model: None,
            language: None,
            output_format: ModeOutputFormat::Email,
            hotkey: None,
        },
        Mode {
            id: "default_mode_notes".to_string(),
            name: "Notes".to_string(),
            prompt: "<transcript>\n${output}\n</transcript>\n\nRewrite the transcript above as concise notes.".to_string(),
            provider_id: None,
            model: None,
            language: None,
            output_format: ModeOutputFormat::BulletList,
            hotkey: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mode() -> Mode {
        Mode {
            id: "m1".to_string(),
            name: "Email".to_string(),
            prompt: "Rewrite as an email.".to_string(),
            provider_id: Some("openai".to_string()),
            model: Some("gpt-4o-mini".to_string()),
            language: Some("en".to_string()),
            output_format: ModeOutputFormat::Email,
            hotkey: None,
        }
    }

    #[test]
    fn no_active_mode_uses_global_prompt_with_style() {
        let resolved = resolve(None, "Global prompt.", Some("Be formal."));
        assert_eq!(resolved.prompt, "Global prompt.\n\nBe formal.");
        assert_eq!(resolved.provider_id, None);
        assert_eq!(resolved.model, None);
        assert_eq!(resolved.language, None);
    }

    #[test]
    fn no_active_mode_no_style_leaves_prompt_untouched() {
        let resolved = resolve(None, "Global prompt.", None);
        assert_eq!(resolved.prompt, "Global prompt.");
    }

    #[test]
    fn active_mode_overrides_prompt_provider_model_language() {
        let mode = sample_mode();
        let resolved = resolve(Some(&mode), "Global prompt.", None);
        assert!(resolved.prompt.starts_with("Rewrite as an email."));
        assert!(resolved.prompt.contains("professional email"));
        assert_eq!(resolved.provider_id, Some("openai".to_string()));
        assert_eq!(resolved.model, Some("gpt-4o-mini".to_string()));
        assert_eq!(resolved.language, Some("en".to_string()));
    }

    #[test]
    fn active_mode_still_appends_style_on_top_of_mode_prompt() {
        let mode = sample_mode();
        let resolved = resolve(Some(&mode), "Global prompt.", Some("Be casual."));
        assert!(resolved.prompt.ends_with("Be casual."));
        assert!(!resolved.prompt.contains("Global prompt."));
    }

    #[test]
    fn plain_output_format_adds_no_instruction() {
        let mut mode = sample_mode();
        mode.output_format = ModeOutputFormat::Plain;
        mode.language = None;
        mode.prompt = "Base prompt.".to_string();
        let resolved = resolve(Some(&mode), "Global.", None);
        assert_eq!(resolved.prompt, "Base prompt.");
    }

    #[test]
    fn bullet_list_output_format_appends_instruction() {
        let mut mode = sample_mode();
        mode.output_format = ModeOutputFormat::BulletList;
        mode.prompt = "Base prompt.".to_string();
        let resolved = resolve(Some(&mode), "Global.", None);
        assert!(resolved.prompt.contains("bullet list"));
    }

    #[test]
    fn find_mode_looks_up_by_id() {
        let modes = vec![sample_mode()];
        assert!(find_mode(&modes, "m1").is_some());
        assert!(find_mode(&modes, "missing").is_none());
    }

    #[test]
    fn mode_binding_id_round_trips() {
        let binding_id = mode_binding_id("m1");
        assert_eq!(binding_id, "mode:m1");
        assert_eq!(mode_id_from_binding(&binding_id), Some("m1"));
        assert_eq!(mode_id_from_binding("transcribe"), None);
    }

    #[test]
    fn default_modes_seed_default_email_notes() {
        let modes = default_modes();
        assert_eq!(modes.len(), 3);
        assert_eq!(modes[0].name, "Default");
        assert_eq!(modes[1].name, "Email");
        assert_eq!(modes[2].name, "Notes");
    }
}
