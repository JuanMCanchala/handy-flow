//! Pure question detection for the profile copilot, EN and ES.
//!
//! Approach adapted from the question-detection heuristics in NexQ
//! (https://github.com/naxhq/NexQ, MIT licensed): a segment is treated as a
//! question if it ends with a question mark, opens with a Spanish inverted
//! question mark, starts with a common interrogative word, or contains an
//! interview-style phrase ("tell me about", "cuéntame", ...). This module
//! only inspects the closed transcript text; it fires after the live
//! subtitles segmenter has already decided the segment is finished.

/// English interrogative words that start a question.
const EN_INTERROGATIVE_STARTS: &[&str] = &[
    "what", "why", "how", "when", "where", "who", "whom", "whose", "which",
    "do", "does", "did", "is", "are", "was", "were", "can", "could", "will",
    "would", "should", "have", "has", "had",
];

/// Spanish interrogative words that start a question (accents stripped before
/// matching, see `strip_accents`).
const ES_INTERROGATIVE_STARTS: &[&str] = &[
    "que", "por que", "porque", "como", "cuando", "donde", "quien", "quienes",
    "cual", "cuales", "cuanto", "cuanta", "cuantos", "cuantas",
];

/// Interview-style phrases that imply a question even without a `?`, checked
/// as substrings anywhere in the (lowercased, accent-stripped) segment.
const INTERVIEW_PHRASES: &[&str] = &[
    // English
    "tell me about",
    "walk me through",
    "why do you",
    "why did you",
    "why are you",
    "why would you",
    "what makes you",
    "describe a time",
    "give me an example",
    "can you tell me",
    "can you describe",
    "can you explain",
    // Spanish (accents already stripped from this list)
    "cuentame",
    "hablame de",
    "hablame sobre",
    "cuentame sobre",
    "cuentame acerca de",
    "puedes hablarme",
    "puedes contarme",
    "puedes explicarme",
    "podrias hablarme",
    "podrias contarme",
    "dame un ejemplo",
    "describe una vez",
];

/// Returns true if `segment` reads as a question directed at the user.
///
/// Checks, in order: trailing `?` (including before trailing whitespace or
/// closing punctuation), a leading Spanish `¿`, an interview-style phrase
/// anywhere in the text, and finally whether the first word is a known
/// English or Spanish interrogative.
pub fn is_question(segment: &str) -> bool {
    let trimmed = segment.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.ends_with('?') || trimmed.starts_with('¿') {
        return true;
    }

    let normalized = strip_accents(&trimmed.to_lowercase());

    if INTERVIEW_PHRASES
        .iter()
        .any(|phrase| normalized.contains(phrase))
    {
        return true;
    }

    let first_word = normalized
        .split(|c: char| !c.is_alphanumeric())
        .find(|w| !w.is_empty())
        .unwrap_or("");

    if first_word.is_empty() {
        return false;
    }

    if EN_INTERROGATIVE_STARTS.contains(&first_word) {
        return true;
    }

    if ES_INTERROGATIVE_STARTS.contains(&first_word) {
        return true;
    }
    // Multi-word interrogative starts like "por que" need the first two words.
    let mut words = normalized.split_whitespace();
    if let (Some(w1), Some(w2)) = (words.next(), words.next()) {
        let first_two = format!("{w1} {w2}");
        if ES_INTERROGATIVE_STARTS.contains(&first_two.as_str()) {
            return true;
        }
    }

    false
}

/// Strips common Spanish accents/diacritics so interrogative word lists don't
/// need every accented variant.
fn strip_accents(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' => 'u',
            'ü' => 'u',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_question_mark_is_detected() {
        assert!(is_question("What is your greatest strength?"));
        assert!(is_question("is this your final answer? "));
    }

    #[test]
    fn spanish_question_mark_is_detected() {
        assert!(is_question("¿Cuál es tu mayor fortaleza?"));
        assert!(is_question("¿por qué dejaste tu trabajo anterior?"));
    }

    #[test]
    fn english_interrogative_start_without_question_mark_is_detected() {
        assert!(is_question("Why do you want to work here"));
        assert!(is_question("How would you handle a difficult teammate"));
        assert!(is_question("Can you describe your last project"));
    }

    #[test]
    fn spanish_interrogative_start_without_question_mark_is_detected() {
        assert!(is_question("Como manejarias un conflicto en el equipo"));
        assert!(is_question("Por que dejaste tu trabajo anterior"));
        assert!(is_question("Cuanto tiempo estuviste en tu ultimo puesto"));
    }

    #[test]
    fn interview_phrases_are_detected_in_english() {
        assert!(is_question("Tell me about a time you failed"));
        assert!(is_question("Walk me through your resume"));
        assert!(is_question("Give me an example of leadership"));
    }

    #[test]
    fn interview_phrases_are_detected_in_spanish() {
        assert!(is_question("Cuéntame sobre tu experiencia en ventas"));
        assert!(is_question("Háblame de tu último proyecto"));
        assert!(is_question("Dame un ejemplo de trabajo en equipo"));
    }

    #[test]
    fn statements_are_not_questions() {
        assert!(!is_question("I have five years of experience in Rust."));
        assert!(!is_question(
            "Tengo cinco años de experiencia en desarrollo."
        ));
        assert!(!is_question("Thanks for having me today."));
        assert!(!is_question("Gracias por la oportunidad."));
    }

    #[test]
    fn empty_or_whitespace_segments_are_not_questions() {
        assert!(!is_question(""));
        assert!(!is_question("   "));
    }
}
