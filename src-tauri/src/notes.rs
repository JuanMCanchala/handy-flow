//! Note generation from transcripts (Granola-style): pure chunking, prompt
//! building, and action-item parsing. The actual LLM call is made by
//! `commands::notes` via `settings::resolve_llm_target` + `llm_client`.
//!
//! Long transcripts are split into fixed-size chunks (`chunk_transcript`),
//! each summarized independently ("map"), then the per-chunk summaries are
//! combined with the template prompt into one final pass ("reduce") so the
//! whole transcript fits within the model's context regardless of length.

use serde::{Deserialize, Serialize};
use specta::Type;

/// A user-defined (or seeded) note-generation template.
#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct NoteTemplate {
    pub id: String,
    pub name: String,
    pub prompt: String,
}

/// A single action item parsed from generated notes markdown.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Type)]
pub struct ActionItem {
    pub text: String,
    pub checked: bool,
}

/// Target size (in characters) of each chunk when splitting a long
/// transcript. Chunks are split on word boundaries so words are never cut
/// in half.
const CHUNK_TARGET_CHARS: usize = 8_000;

/// Splits `transcript` into chunks of roughly `CHUNK_TARGET_CHARS`
/// characters, breaking only on whitespace so words stay intact. Returns a
/// single chunk (the whole transcript) when it is empty or already at or
/// below the target size.
pub fn chunk_transcript(transcript: &str) -> Vec<String> {
    if transcript.len() <= CHUNK_TARGET_CHARS {
        return vec![transcript.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    for word in transcript.split_whitespace() {
        if !current.is_empty() && current.len() + 1 + word.len() > CHUNK_TARGET_CHARS {
            chunks.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    if chunks.is_empty() {
        chunks.push(transcript.to_string());
    }

    chunks
}

/// Builds the prompt for summarizing a single chunk during the "map" phase
/// of map-reduce summarization. `chunk_index`/`total_chunks` are 0-based /
/// count, included so the model knows this is a partial excerpt.
pub fn build_map_prompt(chunk: &str, chunk_index: usize, total_chunks: usize) -> String {
    format!(
        "This is part {} of {} of a longer transcript. Summarize the key points, \
decisions, and any action items mentioned in this excerpt. Be concise but do not \
omit specific facts, names, or numbers.\n\n<transcript_part>\n{}\n</transcript_part>",
        chunk_index + 1,
        total_chunks,
        chunk
    )
}

/// Builds the final prompt sent to the LLM: the template's own prompt plus
/// either the full transcript (short transcripts) or the concatenated
/// per-chunk summaries from the "map" phase (long transcripts, "reduce").
pub fn build_notes_prompt(template_prompt: &str, transcript_or_summaries: &str) -> String {
    format!(
        "{}\n\n<transcript>\n{}\n</transcript>\n\nReturn the notes as markdown. \
Use a \"## Action Items\" section with a markdown checklist (`- [ ] ...`) for any \
action items, tasks, or follow-ups mentioned, even if the section is empty.",
        template_prompt.trim(),
        transcript_or_summaries.trim()
    )
}

/// Parses `- [ ]` / `- [x]` checklist lines out of generated markdown notes,
/// in document order. Matching is case-insensitive for the checkbox marker
/// and tolerant of leading whitespace (nested lists).
pub fn parse_action_items(markdown: &str) -> Vec<ActionItem> {
    markdown
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let rest = trimmed
                .strip_prefix("- [")
                .or_else(|| trimmed.strip_prefix("* ["))?;
            let (marker, rest) = rest.split_once(']')?;
            let text = rest.trim();
            if text.is_empty() {
                return None;
            }
            let checked = matches!(marker.trim(), "x" | "X");
            Some(ActionItem {
                text: text.to_string(),
                checked,
            })
        })
        .collect()
}

/// Re-renders `markdown` with its action-item checklist lines' checked state
/// replaced by `items`, matched positionally (nth checklist line in the
/// document corresponds to `items[n]`). Non-checklist lines and any items
/// beyond what the markdown actually contains are left untouched.
pub fn apply_action_items(markdown: &str, items: &[ActionItem]) -> String {
    let mut item_index = 0;

    markdown
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            let indent_len = line.len() - trimmed.len();
            let indent = &line[..indent_len];

            let bullet = if trimmed.starts_with("- [") {
                "-"
            } else if trimmed.starts_with("* [") {
                "*"
            } else {
                return line.to_string();
            };

            let after_bracket = &trimmed[bullet.len() + 2..];
            let Some((_, rest)) = after_bracket.split_once(']') else {
                return line.to_string();
            };
            let text = rest.trim();
            if text.is_empty() {
                return line.to_string();
            }

            let Some(item) = items.get(item_index) else {
                return line.to_string();
            };
            item_index += 1;

            let marker = if item.checked { "x" } else { " " };
            format!("{indent}{bullet} [{marker}] {text}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_transcript_returns_single_chunk_for_short_text() {
        let chunks = chunk_transcript("hello world");
        assert_eq!(chunks, vec!["hello world".to_string()]);
    }

    #[test]
    fn chunk_transcript_returns_single_chunk_for_empty_text() {
        let chunks = chunk_transcript("");
        assert_eq!(chunks, vec!["".to_string()]);
    }

    #[test]
    fn chunk_transcript_splits_long_text_on_word_boundaries() {
        let word = "word ";
        let transcript = word.repeat(3000); // ~15000 chars, over the target
        let chunks = chunk_transcript(&transcript);

        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.len() <= CHUNK_TARGET_CHARS + "word".len());
            assert!(!chunk.starts_with(' '));
            assert!(!chunk.ends_with(' '));
        }

        // No words were dropped or duplicated.
        let total_words: usize = chunks
            .iter()
            .map(|c| c.split_whitespace().count())
            .sum();
        assert_eq!(total_words, 3000);
    }

    #[test]
    fn build_map_prompt_includes_part_numbers_and_chunk_text() {
        let prompt = build_map_prompt("some excerpt", 1, 5);
        assert!(prompt.contains("part 2 of 5"));
        assert!(prompt.contains("some excerpt"));
    }

    #[test]
    fn build_notes_prompt_includes_template_and_transcript() {
        let prompt = build_notes_prompt("Summarize this meeting.", "we discussed the roadmap");
        assert!(prompt.contains("Summarize this meeting."));
        assert!(prompt.contains("we discussed the roadmap"));
        assert!(prompt.contains("Action Items"));
    }

    #[test]
    fn parse_action_items_finds_unchecked_and_checked_items() {
        let markdown = "## Action Items\n- [ ] Send follow-up email\n- [x] Book the room\n- [ ] Draft agenda\n";
        let items = parse_action_items(markdown);
        assert_eq!(
            items,
            vec![
                ActionItem {
                    text: "Send follow-up email".to_string(),
                    checked: false
                },
                ActionItem {
                    text: "Book the room".to_string(),
                    checked: true
                },
                ActionItem {
                    text: "Draft agenda".to_string(),
                    checked: false
                },
            ]
        );
    }

    #[test]
    fn parse_action_items_ignores_non_checklist_lines() {
        let markdown = "## Summary\nWe discussed the roadmap.\n- A plain bullet\n";
        assert_eq!(parse_action_items(markdown), vec![]);
    }

    #[test]
    fn parse_action_items_supports_star_bullets_and_uppercase_x() {
        let markdown = "* [X] Ship the release\n";
        let items = parse_action_items(markdown);
        assert_eq!(
            items,
            vec![ActionItem {
                text: "Ship the release".to_string(),
                checked: true
            }]
        );
    }

    #[test]
    fn apply_action_items_updates_checked_state_positionally() {
        let markdown = "## Action Items\n- [ ] Send email\n- [ ] Book room\n";
        let items = vec![
            ActionItem {
                text: "Send email".to_string(),
                checked: true,
            },
            ActionItem {
                text: "Book room".to_string(),
                checked: false,
            },
        ];
        let updated = apply_action_items(markdown, &items);
        assert_eq!(
            updated,
            "## Action Items\n- [x] Send email\n- [ ] Book room"
        );
    }

    #[test]
    fn apply_action_items_leaves_other_lines_untouched() {
        let markdown = "## Summary\nSome notes here.\n- [ ] A task\n";
        let items = vec![ActionItem {
            text: "A task".to_string(),
            checked: true,
        }];
        let updated = apply_action_items(markdown, &items);
        assert_eq!(updated, "## Summary\nSome notes here.\n- [x] A task");
    }

    #[test]
    fn round_trip_parse_then_apply_preserves_text() {
        let markdown = "- [ ] First\n- [ ] Second\n";
        let mut items = parse_action_items(markdown);
        items[0].checked = true;
        let updated = apply_action_items(markdown, &items);
        assert_eq!(parse_action_items(&updated), items);
    }
}
