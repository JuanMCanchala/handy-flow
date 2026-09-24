//! Query sanitization and prompt building for "Ask your history": a search
//! box on Home that runs a SQLite FTS5 query over transcription history and,
//! for "Ask", stuffs the top-k matches into a prompt for the configured LLM
//! (`settings::resolve_llm_target` + `llm_client::send_chat_completion`).
//! Kept free of I/O and Tauri types so it is easy to unit test; callers
//! (see `managers::history::HistoryManager` and `commands::history`) run the
//! actual FTS5 query and LLM call.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Number of top FTS matches stuffed into the "Ask" prompt.
pub const ASK_TOP_K: usize = 8;

/// A single history entry matched by a search/ask query, enough to render a
/// result with a link back to the source entry.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct SearchResult {
    pub id: i64,
    pub title: String,
    pub snippet: String,
}

/// Sanitizes a raw user query for use in a SQLite FTS5 `MATCH` expression.
///
/// FTS5 query syntax treats characters like `"`, `*`, `:`, `(`, `)`, `-`,
/// `^` as operators; passing raw user input as the MATCH string can throw a
/// syntax error or change query semantics (e.g. a leading `-` negates the
/// next term). This strips FTS5 operator characters, splits the remainder
/// into words, and rejoins them as a sequence of quoted-phrase tokens so the
/// FTS5 engine treats each word as a literal, ANDed together.
///
/// Returns `None` when the query has no usable words (e.g. empty, or only
/// operator characters).
pub fn sanitize_fts_query(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() || c == '\'' {
                c
            } else {
                ' '
            }
        })
        .collect();

    let words: Vec<String> = cleaned
        .split_whitespace()
        .map(|w| w.replace('"', ""))
        .filter(|w| !w.is_empty())
        .collect();

    if words.is_empty() {
        return None;
    }

    Some(
        words
            .iter()
            .map(|w| format!("\"{w}\""))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Builds the prompt sent to the LLM for "Ask your history": the user's
/// question plus the top-k FTS matches, each labeled with its entry id so
/// the model (and the UI) can link back to the source.
///
/// `matches` should already be truncated to `ASK_TOP_K` by the caller; this
/// function does not re-truncate so tests can exercise boundary behavior
/// directly.
pub fn build_ask_prompt(question: &str, matches: &[SearchResult]) -> String {
    let mut prompt = String::new();

    prompt.push_str(
        "You are answering a question using the user's own dictation history as the \
only source of truth. Use only the excerpts below; if they don't contain the \
answer, say so plainly instead of guessing.\n\n",
    );

    if matches.is_empty() {
        prompt.push_str("(no matching history entries found)\n\n");
    } else {
        prompt.push_str("Matching history excerpts:\n");
        for m in matches {
            prompt.push_str(&format!("[{}] {}: {}\n", m.id, m.title, m.snippet));
        }
        prompt.push('\n');
    }

    prompt.push_str("Question: \"");
    prompt.push_str(question.trim());
    prompt.push_str("\"\n\n");

    prompt.push_str(
        "Reply with a concise answer grounded in the excerpts above. When you use an \
excerpt, cite it inline using its bracketed id, e.g. [3].",
    );

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_plain_words() {
        assert_eq!(
            sanitize_fts_query("codefest astra").as_deref(),
            Some("\"codefest\" \"astra\"")
        );
    }

    #[test]
    fn strips_fts5_operator_characters() {
        assert_eq!(
            sanitize_fts_query("foo* OR -bar : (baz)").as_deref(),
            Some("\"foo\" \"OR\" \"bar\" \"baz\"")
        );
    }

    #[test]
    fn strips_quotes_inside_words() {
        assert_eq!(
            sanitize_fts_query("say \"hello\" now").as_deref(),
            Some("\"say\" \"hello\" \"now\"")
        );
    }

    #[test]
    fn empty_query_returns_none() {
        assert_eq!(sanitize_fts_query(""), None);
        assert_eq!(sanitize_fts_query("   "), None);
    }

    #[test]
    fn only_operator_characters_returns_none() {
        assert_eq!(sanitize_fts_query("*** ---"), None);
    }

    #[test]
    fn preserves_apostrophes_in_contractions() {
        assert_eq!(
            sanitize_fts_query("don't stop").as_deref(),
            Some("\"don't\" \"stop\"")
        );
    }

    #[test]
    fn prompt_includes_question_and_matches_with_ids() {
        let matches = vec![
            SearchResult {
                id: 3,
                title: "March 1".to_string(),
                snippet: "met with codefest astra team".to_string(),
            },
            SearchResult {
                id: 7,
                title: "March 5".to_string(),
                snippet: "followed up with codefest astra".to_string(),
            },
        ];
        let prompt = build_ask_prompt("when did I meet codefest astra?", &matches);
        assert!(prompt.contains("when did I meet codefest astra?"));
        assert!(prompt.contains("[3] March 1: met with codefest astra team"));
        assert!(prompt.contains("[7] March 5: followed up with codefest astra"));
    }

    #[test]
    fn prompt_notes_when_no_matches_found() {
        let prompt = build_ask_prompt("anything about llamas?", &[]);
        assert!(prompt.contains("no matching history entries found"));
    }

    #[test]
    fn top_k_constant_is_eight() {
        assert_eq!(ASK_TOP_K, 8);
    }
}
