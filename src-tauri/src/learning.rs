//! Pure word-level diff and candidate extraction for the self-learning
//! dictionary. When a user edits a history entry, `extract_candidates`
//! compares the original and edited text and returns replacement candidates
//! ("codefest atastra" -> "Codefest Astra") worth remembering, ignoring
//! noise like case-only or punctuation-only changes. Kept free of I/O and
//! Tauri types so it is easy to unit test; callers (see
//! `managers::history::HistoryManager`) persist candidates and bump hit
//! counters.

use serde::{Deserialize, Serialize};
use specta::Type;

/// A single suggested replacement derived from a diff between original and
/// edited text.
#[derive(Clone, Debug, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct Candidate {
    pub from: String,
    pub to: String,
}

/// Splits text into lowercase-comparable word tokens (alphanumeric runs),
/// discarding punctuation and whitespace, while keeping the original-cased
/// word alongside its normalized form for comparison.
fn tokenize(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '\u{2019}'))
        .filter(|w| !w.is_empty())
        .collect()
}

fn normalize(word: &str) -> String {
    word.to_lowercase()
}

/// Strips case and punctuation-only differences: returns true when the two
/// words are the same once lowercased.
fn is_case_only_change(a: &str, b: &str) -> bool {
    normalize(a) == normalize(b)
}

/// Longest Common Subsequence-based word alignment. Returns a list of
/// operations: `Equal` for words present (case-insensitively) in both
/// sequences, and `Replace` for runs that differ.
enum DiffOp<'a> {
    Equal,
    Replace(Vec<&'a str>, Vec<&'a str>),
}

/// Computes a simple word-level diff between `original` and `edited` using
/// LCS on normalized (lowercased) words, then groups the non-matching runs
/// into `Replace` ops.
fn diff_words<'a>(original: &[&'a str], edited: &[&'a str]) -> Vec<DiffOp<'a>> {
    let n = original.len();
    let m = edited.len();

    // dp[i][j] = length of LCS of original[i..] and edited[j..]
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            if is_case_only_change(original[i], edited[j]) {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut ops = Vec::new();
    let mut i = 0;
    let mut j = 0;
    let mut pending_orig: Vec<&str> = Vec::new();
    let mut pending_edit: Vec<&str> = Vec::new();

    macro_rules! flush {
        () => {
            if !pending_orig.is_empty() || !pending_edit.is_empty() {
                ops.push(DiffOp::Replace(
                    std::mem::take(&mut pending_orig),
                    std::mem::take(&mut pending_edit),
                ));
            }
        };
    }

    while i < n && j < m {
        if is_case_only_change(original[i], edited[j]) {
            flush!();
            ops.push(DiffOp::Equal);
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            pending_orig.push(original[i]);
            i += 1;
        } else {
            pending_edit.push(edited[j]);
            j += 1;
        }
    }
    while i < n {
        pending_orig.push(original[i]);
        i += 1;
    }
    while j < m {
        pending_edit.push(edited[j]);
        j += 1;
    }
    flush!();

    ops
}

/// Extracts self-learning dictionary candidates by diffing `original` and
/// `edited` text word-by-word.
///
/// Rules:
/// - Case-only changes (e.g. "handy" -> "Handy" as a whole-word rewrite
///   used purely for casing) and punctuation-only changes are ignored: they
///   produce no candidates.
/// - A run of replaced words on each side becomes a candidate `from` -> `to`
///   pair, provided both sides are non-empty (pure insertions/deletions of
///   words aren't corrections of a mis-transcribed term, so they're
///   skipped).
/// - Unrelated full-sentence rewrites (edited text sharing no common words
///   at all with the original around the change) are still surfaced as a
///   single candidate per contiguous diff run; the caller decides whether
///   accumulated hits justify surfacing it. Extremely large rewrites (more
///   than `MAX_CANDIDATE_WORDS` words on either side) are dropped as noise.
const MAX_CANDIDATE_WORDS: usize = 6;

pub fn extract_candidates(original: &str, edited: &str) -> Vec<Candidate> {
    let original_words = tokenize(original);
    let edited_words = tokenize(edited);

    let ops = diff_words(&original_words, &edited_words);

    let mut candidates = Vec::new();
    for op in ops {
        if let DiffOp::Replace(from_words, to_words) = op {
            if from_words.is_empty() || to_words.is_empty() {
                continue;
            }
            if from_words.len() > MAX_CANDIDATE_WORDS || to_words.len() > MAX_CANDIDATE_WORDS {
                continue;
            }
            let from = from_words.join(" ");
            let to = to_words.join(" ");
            if is_case_only_change(&from, &to) {
                continue;
            }
            candidates.push(Candidate { from, to });
        }
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_text_yields_no_candidates() {
        assert_eq!(extract_candidates("hello world", "hello world"), vec![]);
    }

    #[test]
    fn case_only_change_is_ignored() {
        assert_eq!(extract_candidates("handy is great", "Handy is great"), vec![]);
    }

    #[test]
    fn punctuation_only_change_is_ignored() {
        assert_eq!(
            extract_candidates("hello world", "hello, world!"),
            vec![]
        );
    }

    #[test]
    fn single_word_correction_is_detected() {
        // "codefest" matches case-insensitively on both sides, so only the
        // mis-transcribed word itself becomes a candidate.
        assert_eq!(
            extract_candidates("i went to codefest atastra", "i went to Codefest Astra"),
            vec![Candidate {
                from: "atastra".to_string(),
                to: "Astra".to_string(),
            }]
        );
    }

    #[test]
    fn multi_word_phrase_correction_is_detected_when_no_word_matches() {
        assert_eq!(
            extract_candidates("i went to codefezt atastra", "i went to Codefest Astra"),
            vec![Candidate {
                from: "codefezt atastra".to_string(),
                to: "Codefest Astra".to_string(),
            }]
        );
    }

    #[test]
    fn multi_word_name_correction_is_detected() {
        assert_eq!(
            extract_candidates(
                "meet with jon apleseed tomorrow",
                "meet with Jonathan Appleseed tomorrow"
            ),
            vec![Candidate {
                from: "jon apleseed".to_string(),
                to: "Jonathan Appleseed".to_string(),
            }]
        );
    }

    #[test]
    fn pure_insertion_is_ignored() {
        // Edited text adds a word with no corresponding original word to
        // replace; not a mis-transcription correction.
        assert_eq!(extract_candidates("hello world", "hello there world"), vec![]);
    }

    #[test]
    fn pure_deletion_is_ignored() {
        assert_eq!(extract_candidates("hello there world", "hello world"), vec![]);
    }

    #[test]
    fn unrelated_full_rewrite_is_ignored_when_too_large() {
        let original = "the quick brown fox jumps over the lazy dog today";
        let edited = "a completely different sentence about something else entirely now";
        assert_eq!(extract_candidates(original, edited), vec![]);
    }

    #[test]
    fn empty_strings_yield_no_candidates() {
        assert_eq!(extract_candidates("", ""), vec![]);
    }
}
