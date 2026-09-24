//! Snippet expansion: replaces user-defined trigger phrases with their
//! expansion text in a final transcription, before it is pasted.

use crate::settings::Snippet;

/// Returns `true` if `c` is considered a word character for trigger matching
/// (alphanumeric or underscore), so punctuation surrounding a trigger doesn't
/// prevent a match, but adjacent letters/digits do (no partial-word matches).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Case-insensitively replaces every whole-word occurrence of each snippet's
/// `trigger` in `text` with its `expansion`. Matches are tolerant of
/// surrounding punctuation but require word boundaries on both sides, so a
/// trigger never matches inside a larger word. Snippets are applied in order;
/// empty triggers are skipped. Returns the text unchanged if there are no
/// snippets.
pub fn apply_snippets(text: &str, snippets: &[Snippet]) -> String {
    if snippets.is_empty() {
        return text.to_string();
    }

    let mut result = text.to_string();

    for snippet in snippets {
        let trigger = snippet.trigger.trim();
        if trigger.is_empty() {
            continue;
        }
        result = replace_trigger(&result, trigger, &snippet.expansion);
    }

    result
}

/// Replaces whole-word, case-insensitive occurrences of `trigger` in `text`
/// with `expansion`.
fn replace_trigger(text: &str, trigger: &str, expansion: &str) -> String {
    let text_chars: Vec<char> = text.chars().collect();
    let trigger_lower: Vec<char> = trigger.to_lowercase().chars().collect();
    let trigger_len = trigger_lower.len();

    if trigger_len == 0 {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len());
    let mut i = 0;

    while i < text_chars.len() {
        let is_boundary_before = i == 0 || !is_word_char(text_chars[i - 1]);

        if is_boundary_before && i + trigger_len <= text_chars.len() {
            let candidate: String = text_chars[i..i + trigger_len]
                .iter()
                .collect::<String>()
                .to_lowercase();

            let is_boundary_after =
                i + trigger_len == text_chars.len() || !is_word_char(text_chars[i + trigger_len]);

            if is_boundary_after && candidate.chars().collect::<Vec<char>>() == trigger_lower {
                result.push_str(expansion);
                i += trigger_len;
                continue;
            }
        }

        result.push(text_chars[i]);
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet(trigger: &str, expansion: &str) -> Snippet {
        Snippet {
            id: trigger.to_string(),
            trigger: trigger.to_string(),
            expansion: expansion.to_string(),
        }
    }

    #[test]
    fn empty_snippet_list_returns_text_unchanged() {
        assert_eq!(apply_snippets("hello world", &[]), "hello world");
    }

    #[test]
    fn single_snippet_is_expanded() {
        let snippets = vec![snippet("brb", "be right back")];
        assert_eq!(
            apply_snippets("I will brb soon", &snippets),
            "I will be right back soon"
        );
    }

    #[test]
    fn multiple_snippets_are_all_expanded() {
        let snippets = vec![snippet("brb", "be right back"), snippet("omw", "on my way")];
        assert_eq!(
            apply_snippets("brb but also omw", &snippets),
            "be right back but also on my way"
        );
    }

    #[test]
    fn matching_is_case_insensitive() {
        let snippets = vec![snippet("brb", "be right back")];
        assert_eq!(apply_snippets("BRB now", &snippets), "be right back now");
        assert_eq!(apply_snippets("Brb now", &snippets), "be right back now");
    }

    #[test]
    fn surrounding_punctuation_is_tolerated() {
        let snippets = vec![snippet("brb", "be right back")];
        assert_eq!(apply_snippets("brb, ok?", &snippets), "be right back, ok?");
        assert_eq!(apply_snippets("(brb)", &snippets), "(be right back)");
    }

    #[test]
    fn no_partial_word_match() {
        let snippets = vec![snippet("brb", "be right back")];
        assert_eq!(apply_snippets("brbing along", &snippets), "brbing along");
        assert_eq!(apply_snippets("superbrb", &snippets), "superbrb");
    }

    #[test]
    fn empty_trigger_is_skipped() {
        let snippets = vec![
            snippet("", "should not appear"),
            snippet("brb", "be right back"),
        ];
        assert_eq!(apply_snippets("brb", &snippets), "be right back");
    }
}
