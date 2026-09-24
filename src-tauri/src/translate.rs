//! Translate hotkey: dictate, then paste the translation instead of the
//! transcript. Reuses command mode's LLM call — the transcript is passed as
//! the "selection" and this module only builds the instruction text.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "snake_case")]
pub enum TranslationTarget {
    #[default]
    Auto,
    En,
    Es,
}

/// Builds the command-mode instruction for translating `text` according to
/// `target`. In `Auto` mode, Spanish text is translated to English and
/// anything else is translated to Spanish (EN\u2194ES focus).
pub fn build_translate_instruction(text: &str, target: TranslationTarget) -> String {
    let language = match target {
        TranslationTarget::En => "English",
        TranslationTarget::Es => "Spanish",
        TranslationTarget::Auto => {
            if looks_like_spanish(text) {
                "English"
            } else {
                "Spanish"
            }
        }
    };
    format!("Translate the text to {language}. Output only the translation.")
}

/// Rough heuristic for detecting Spanish text: presence of accented vowels,
/// ñ, or common Spanish-only stop words not used in English.
fn looks_like_spanish(text: &str) -> bool {
    let lower = text.to_lowercase();
    if lower.chars().any(|c| matches!(c, 'á' | 'é' | 'í' | 'ó' | 'ú' | 'ñ' | '¿' | '¡')) {
        return true;
    }
    const SPANISH_WORDS: &[&str] = &[
        "el", "la", "los", "las", "de", "que", "y", "en", "un", "una", "es", "por", "para",
        "con", "no", "se", "su", "al", "lo", "como", "mas", "pero", "esta", "este",
    ];
    let words: Vec<&str> = lower.split_whitespace().collect();
    if words.is_empty() {
        return false;
    }
    let spanish_hits = words
        .iter()
        .filter(|w| SPANISH_WORDS.contains(&w.trim_matches(|c: char| !c.is_alphanumeric())))
        .count();
    spanish_hits * 3 >= words.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_target_ignores_source_language() {
        assert_eq!(
            build_translate_instruction("hola mundo", TranslationTarget::En),
            "Translate the text to English. Output only the translation."
        );
        assert_eq!(
            build_translate_instruction("hello world", TranslationTarget::Es),
            "Translate the text to Spanish. Output only the translation."
        );
    }

    #[test]
    fn auto_detects_spanish_and_translates_to_english() {
        assert_eq!(
            build_translate_instruction(
                "Hola, ¿cómo estás? Espero que estés bien.",
                TranslationTarget::Auto
            ),
            "Translate the text to English. Output only the translation."
        );
    }

    #[test]
    fn auto_defaults_non_spanish_to_spanish() {
        assert_eq!(
            build_translate_instruction("Hello, how are you today?", TranslationTarget::Auto),
            "Translate the text to Spanish. Output only the translation."
        );
    }

    #[test]
    fn auto_handles_empty_text() {
        assert_eq!(
            build_translate_instruction("", TranslationTarget::Auto),
            "Translate the text to Spanish. Output only the translation."
        );
    }
}
