//! Builds the translation prompt sent to the configured LLM
//! (`settings::resolve_llm_target` + `llm_client::send_chat_completion`).
//!
//! Each segment is translated with the previous 2-3 segments as context, so
//! pronouns and terminology stay consistent across a live conversation.
//! Source language is auto-detected upstream (by the transcription engine);
//! this module only decides the target language (the other of EN/ES) and
//! formats the prompt.

/// Max number of prior segments included as context.
pub const MAX_CONTEXT_SEGMENTS: usize = 3;

/// The two languages this feature translates between.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtitleLanguage {
    En,
    Es,
}

impl SubtitleLanguage {
    /// The other language in the EN<->ES pair.
    pub fn other(self) -> Self {
        match self {
            SubtitleLanguage::En => SubtitleLanguage::Es,
            SubtitleLanguage::Es => SubtitleLanguage::En,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            SubtitleLanguage::En => "en",
            SubtitleLanguage::Es => "es",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SubtitleLanguage::En => "English",
            SubtitleLanguage::Es => "Spanish",
        }
    }

    /// Best-effort classification of a transcript's detected language code
    /// (e.g. "en", "es", "en-US") into one of the two subtitle languages.
    /// Defaults to English when the code doesn't look like Spanish, so an
    /// unrecognized/auto code still produces a sensible target (Spanish).
    pub fn from_code(code: &str) -> Self {
        if code.to_lowercase().starts_with("es") {
            SubtitleLanguage::Es
        } else {
            SubtitleLanguage::En
        }
    }
}

const SPANISH_WORDS: &[&str] = &[
    "el", "la", "los", "las", "de", "que", "y", "en", "un", "una", "es", "por", "con", "para",
    "no", "se", "lo", "como", "pero", "más", "mas", "muy", "está", "esta", "estoy", "yo", "tú",
    "usted", "nosotros", "hola", "gracias", "qué", "cómo", "porque", "también", "sí",
];
const ENGLISH_WORDS: &[&str] = &[
    "the", "and", "of", "to", "is", "in", "that", "it", "you", "for", "on", "with", "are", "this",
    "was", "have", "be", "what", "how", "why", "i", "we", "they", "hello", "thanks", "not", "but",
    "so", "do", "can", "will",
];

/// Guesses whether a transcript is Spanish or English: accents/ñ/¿¡ are a
/// strong Spanish signal, otherwise the more frequent stop-word set wins
/// (ties go to English).
pub fn detect_language(text: &str) -> SubtitleLanguage {
    let lower = text.to_lowercase();
    let accent_hits = lower
        .chars()
        .filter(|c| matches!(c, 'ñ' | 'á' | 'é' | 'í' | 'ó' | 'ú' | '¿' | '¡'))
        .count();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric() && !"ñáéíóúü".contains(c))
        .filter(|w| !w.is_empty())
        .collect();
    let es = words.iter().filter(|w| SPANISH_WORDS.contains(w)).count() + accent_hits * 2;
    let en = words.iter().filter(|w| ENGLISH_WORDS.contains(w)).count();
    if es > en {
        SubtitleLanguage::Es
    } else {
        SubtitleLanguage::En
    }
}

/// One previously-translated segment, kept for context in later prompts.
#[derive(Debug, Clone)]
pub struct ContextSegment {
    pub original: String,
    pub translation: String,
}

/// Build the user prompt for translating `text` into `target`, including up
/// to `MAX_CONTEXT_SEGMENTS` previous segments as context. `context` is
/// ordered oldest-first; only the most recent entries are used.
pub fn build_translation_prompt(
    text: &str,
    target: SubtitleLanguage,
    context: &[ContextSegment],
) -> String {
    let mut prompt = String::new();
    prompt.push_str(&format!(
        "Translate the following live-conversation transcript segment into {}. \
Reply with only the translation, no explanations or quotes.\n",
        target.label()
    ));

    let recent = context
        .iter()
        .rev()
        .take(MAX_CONTEXT_SEGMENTS)
        .rev()
        .collect::<Vec<_>>();

    if !recent.is_empty() {
        prompt.push_str("\nPrevious segments (for context only, do not translate these):\n");
        for segment in recent {
            prompt.push_str(&format!(
                "- \"{}\" -> \"{}\"\n",
                segment.original, segment.translation
            ));
        }
    }

    prompt.push_str(&format!("\nSegment to translate:\n\"{}\"", text));
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_spanish_and_english_segments() {
        assert_eq!(
            detect_language("Hola, ¿cómo estás? Esto es una prueba"),
            SubtitleLanguage::Es
        );
        assert_eq!(
            detect_language("pero no se lo que quiere decir"),
            SubtitleLanguage::Es
        );
        assert_eq!(
            detect_language("Hello, how are you? This is a test"),
            SubtitleLanguage::En
        );
        assert_eq!(
            detect_language("we can do it with the team"),
            SubtitleLanguage::En
        );
        assert_eq!(detect_language(""), SubtitleLanguage::En);
    }

    #[test]
    fn other_language_is_the_opposite_of_the_pair() {
        assert_eq!(SubtitleLanguage::En.other(), SubtitleLanguage::Es);
        assert_eq!(SubtitleLanguage::Es.other(), SubtitleLanguage::En);
    }

    #[test]
    fn from_code_classifies_spanish_variants() {
        assert_eq!(SubtitleLanguage::from_code("es"), SubtitleLanguage::Es);
        assert_eq!(SubtitleLanguage::from_code("ES-MX"), SubtitleLanguage::Es);
    }

    #[test]
    fn from_code_defaults_to_english_for_unknown_or_auto() {
        assert_eq!(SubtitleLanguage::from_code("auto"), SubtitleLanguage::En);
        assert_eq!(SubtitleLanguage::from_code("en"), SubtitleLanguage::En);
        assert_eq!(SubtitleLanguage::from_code("fr"), SubtitleLanguage::En);
    }

    #[test]
    fn prompt_without_context_only_has_instruction_and_segment() {
        let prompt = build_translation_prompt("Hello there", SubtitleLanguage::Es, &[]);
        assert!(prompt.contains("into Spanish"));
        assert!(prompt.contains("\"Hello there\""));
        assert!(!prompt.contains("Previous segments"));
    }

    #[test]
    fn prompt_includes_up_to_three_most_recent_context_segments_in_order() {
        let context = vec![
            ContextSegment {
                original: "one".to_string(),
                translation: "uno".to_string(),
            },
            ContextSegment {
                original: "two".to_string(),
                translation: "dos".to_string(),
            },
            ContextSegment {
                original: "three".to_string(),
                translation: "tres".to_string(),
            },
            ContextSegment {
                original: "four".to_string(),
                translation: "cuatro".to_string(),
            },
        ];

        let prompt = build_translation_prompt("five", SubtitleLanguage::Es, &context);

        assert!(!prompt.contains("\"one\""));
        let idx_two = prompt.find("\"two\"").expect("two present");
        let idx_three = prompt.find("\"three\"").expect("three present");
        let idx_four = prompt.find("\"four\"").expect("four present");
        let idx_segment = prompt.find("Segment to translate").expect("segment marker");

        // Oldest-first ordering preserved among the kept entries.
        assert!(idx_two < idx_three);
        assert!(idx_three < idx_four);
        assert!(idx_four < idx_segment);
    }

    #[test]
    fn prompt_targets_english_when_translating_to_english() {
        let prompt = build_translation_prompt("Hola", SubtitleLanguage::En, &[]);
        assert!(prompt.contains("into English"));
    }
}
