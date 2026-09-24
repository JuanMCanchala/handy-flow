//! Transforms (Wispr Flow style): user-saved named prompts that can be
//! invoked by name while in command mode ("apply bullet list", "make it
//! formal"). Matching reuses the same normalization command mode already
//! applies to the agent name so accents/punctuation/case don't matter.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Words that may precede a transform name in the spoken instruction.
const TRIGGER_WORDS: &[&str] = &["apply", "aplica", "usa", "use"];

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct Transform {
    pub id: String,
    pub name: String,
    pub prompt: String,
}

/// If `instruction` names one of the saved transforms (optionally preceded by
/// a trigger word like "apply"/"aplica"), returns that transform's prompt.
/// Matching is case-, accent- and punctuation-insensitive.
pub fn match_transform<'a>(instruction: &str, transforms: &'a [Transform]) -> Option<&'a str> {
    let normalized = normalize(instruction);
    if normalized.is_empty() {
        return None;
    }

    for transform in transforms {
        let name = normalize(&transform.name);
        if name.is_empty() {
            continue;
        }
        if normalized == name {
            return Some(transform.prompt.as_str());
        }
        for trigger in TRIGGER_WORDS {
            let prefix = format!("{trigger} ");
            if let Some(rest) = normalized.strip_prefix(&prefix) {
                if rest == name {
                    return Some(transform.prompt.as_str());
                }
            }
        }
    }
    None
}

fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .map(fold_accent)
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_transforms() -> Vec<Transform> {
        vec![
            Transform {
                id: "bullet_list".to_string(),
                name: "Bullet list".to_string(),
                prompt: "Rewrite as a bullet list.".to_string(),
            },
            Transform {
                id: "make_formal".to_string(),
                name: "Make it formal".to_string(),
                prompt: "Rewrite in a formal tone.".to_string(),
            },
        ]
    }

    #[test]
    fn exact_name_match() {
        let transforms = sample_transforms();
        assert_eq!(
            match_transform("bullet list", &transforms),
            Some("Rewrite as a bullet list.")
        );
    }

    #[test]
    fn case_accent_and_punctuation_insensitive() {
        let transforms = sample_transforms();
        assert_eq!(
            match_transform("Bullét List!", &transforms),
            Some("Rewrite as a bullet list.")
        );
        assert_eq!(
            match_transform("MAKE IT FORMAL.", &transforms),
            Some("Rewrite in a formal tone.")
        );
    }

    #[test]
    fn trigger_word_prefix_in_english_and_spanish() {
        let transforms = sample_transforms();
        assert_eq!(
            match_transform("apply bullet list", &transforms),
            Some("Rewrite as a bullet list.")
        );
        assert_eq!(
            match_transform("aplica make it formal", &transforms),
            Some("Rewrite in a formal tone.")
        );
    }

    #[test]
    fn non_matching_instruction_returns_none() {
        let transforms = sample_transforms();
        assert_eq!(match_transform("summarize this email", &transforms), None);
        assert_eq!(match_transform("", &transforms), None);
    }
}
