//! Deterministic post-transcription pass that turns spoken editing commands
//! into their intended effect, instead of leaving them as literal text.
//!
//! Supported (English and Spanish), matched case-insensitively as whole
//! words/phrases:
//! - "scratch that" / "delete that" / "borra eso": deletes the sentence or
//!   clause immediately preceding the command. A "sentence or clause" is the
//!   run of text since the previous sentence-ending punctuation (`.`, `!`,
//!   `?`) or clause separator (`,`), or the start of the text if none.
//! - "new line" / "nueva línea" / "nueva linea": inserts a single newline.
//! - "new paragraph" / "nuevo párrafo" / "nuevo parrafo": inserts a blank
//!   line (two newlines).
//! - Spoken punctuation words ("comma"/"coma", "period"/"punto",
//!   "question mark"/"signo de interrogación"): replaced with the literal
//!   symbol, but only when the word is being used as punctuation rather than
//!   as part of an ordinary phrase. We treat a punctuation word as a command
//!   only when it is the last word of the utterance, or is immediately
//!   followed by another recognized command phrase (new line/new
//!   paragraph/scratch that/delete that). This keeps phrases like "punto de
//!   vista" or "point of view" intact, since "punto"/"point" there is
//!   followed by ordinary words, not a command or end of input.
//!
//! "actually X" / "no, X" style corrections are intentionally not handled
//! here; they are left for AI post-processing.

/// Applies all supported voice-edit commands to `text` and returns the
/// resulting string. Idempotent: running it twice on already-clean text
/// (with no remaining command phrases) returns the same output.
pub fn apply_voice_edits(text: &str) -> String {
    let tokens = tokenize(text);
    let commands = parse_commands(&tokens);
    render(&tokens, &commands)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Word,
    Whitespace,
    Punct,
}

#[derive(Debug, Clone)]
struct Token<'a> {
    text: &'a str,
    kind: TokenKind,
}

/// Splits `text` into words, runs of whitespace, and individual punctuation
/// characters, preserving everything so the original string can be
/// reconstructed by concatenation.
fn tokenize(text: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (idx, ch) = chars[i];
        if ch.is_whitespace() {
            if start < idx {
                tokens.push(Token {
                    text: &text[start..idx],
                    kind: TokenKind::Word,
                });
            }
            let ws_start = idx;
            let mut j = i;
            while j < chars.len() && chars[j].1.is_whitespace() {
                j += 1;
            }
            let ws_end = if j < chars.len() {
                chars[j].0
            } else {
                text.len()
            };
            tokens.push(Token {
                text: &text[ws_start..ws_end],
                kind: TokenKind::Whitespace,
            });
            i = j;
            start = ws_end;
        } else if is_word_char(ch) {
            i += 1;
        } else {
            // Punctuation / symbol character: flush pending word, emit punct.
            if start < idx {
                tokens.push(Token {
                    text: &text[start..idx],
                    kind: TokenKind::Word,
                });
            }
            let punct_end = idx + ch.len_utf8();
            tokens.push(Token {
                text: &text[idx..punct_end],
                kind: TokenKind::Punct,
            });
            i += 1;
            start = punct_end;
        }
    }
    if start < text.len() {
        tokens.push(Token {
            text: &text[start..],
            kind: TokenKind::Word,
        });
    }
    tokens
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '\'' || ch == '\u{2019}'
}

/// Indices (into `tokens`) of the word tokens only, in order.
fn word_indices(tokens: &[Token<'_>]) -> Vec<usize> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| t.kind == TokenKind::Word)
        .map(|(i, _)| i)
        .collect()
}

#[derive(Debug, Clone, Copy)]
enum CommandKind {
    /// Delete the preceding sentence/clause. Range is expressed in word
    /// index space (see `word_indices`): [start, end) of the command phrase
    /// itself, plus the deletion start word index.
    ScratchThat,
    NewLine,
    NewParagraph,
    Punct(&'static str),
}

struct Command {
    /// Position in `word_indices` (i.e. the Nth word) where the command
    /// phrase starts.
    word_start: usize,
    /// Position in `word_indices` where the command phrase ends (exclusive).
    word_end: usize,
    kind: CommandKind,
}

/// Recognized multi-word (or single-word) command phrases, lower-cased.
/// Longer phrases are listed first among options for the same word count is
/// not required since we always try the longest match at each position.
fn phrase_table() -> &'static [(&'static [&'static str], CommandKind)] {
    &[
        (&["scratch", "that"], CommandKind::ScratchThat),
        (&["delete", "that"], CommandKind::ScratchThat),
        (&["borra", "eso"], CommandKind::ScratchThat),
        (&["new", "line"], CommandKind::NewLine),
        (&["nueva", "linea"], CommandKind::NewLine),
        (&["new", "paragraph"], CommandKind::NewParagraph),
        (&["nuevo", "parrafo"], CommandKind::NewParagraph),
        (&["comma"], CommandKind::Punct(",")),
        (&["coma"], CommandKind::Punct(",")),
        (&["period"], CommandKind::Punct(".")),
        (&["punto"], CommandKind::Punct(".")),
        (&["question", "mark"], CommandKind::Punct("?")),
        (&["signo", "de", "interrogacion"], CommandKind::Punct("?")),
    ]
}

/// Normalizes a word for matching: lowercases and strips diacritics from the
/// small set of accented letters used by the Spanish phrases above, so
/// "línea"/"linea", "párrafo"/"parrafo", "interrogación"/"interrogacion" all
/// match regardless of accents.
fn normalize_word(word: &str) -> String {
    word.chars()
        .map(|c| match c {
            'á' | 'Á' => 'a',
            'é' | 'É' => 'e',
            'í' | 'Í' => 'i',
            'ó' | 'Ó' => 'o',
            'ú' | 'Ú' | 'ü' | 'Ü' => 'u',
            'ñ' | 'Ñ' => 'n',
            other => other,
        })
        .collect::<String>()
        .to_lowercase()
}

fn is_command_phrase_kind(kind: CommandKind) -> bool {
    !matches!(kind, CommandKind::Punct(_))
}

/// Scans the normalized word sequence and returns all recognized commands,
/// left to right. Matching is greedy at each position (longest phrase wins),
/// and once a command is matched its words are consumed (not reused by a
/// later match).
fn parse_commands(tokens: &[Token<'_>]) -> Vec<Command> {
    let word_idx = word_indices(tokens);
    let normalized: Vec<String> = word_idx
        .iter()
        .map(|&i| normalize_word(tokens[i].text))
        .collect();

    let table = phrase_table();
    let n = normalized.len();
    let mut commands = Vec::new();
    let mut pos = 0;
    while pos < n {
        let mut matched: Option<(usize, CommandKind)> = None; // (len, kind)
        for (phrase, kind) in table {
            let len = phrase.len();
            if pos + len > n {
                continue;
            }
            let matches = (0..len).all(|k| normalized[pos + k] == phrase[k]);
            if matches {
                // Punctuation words are only treated as commands when they
                // end the utterance or are immediately followed by another
                // command phrase (checked after we know what's next).
                if let CommandKind::Punct(_) = kind {
                    let next_pos = pos + len;
                    let is_last = next_pos >= n;
                    let followed_by_command =
                        !is_last && starts_command_at(&normalized, next_pos, table);
                    if !is_last && !followed_by_command {
                        continue;
                    }
                }
                if matched.map(|(l, _)| len > l).unwrap_or(true) {
                    matched = Some((len, *kind));
                }
            }
        }
        if let Some((len, kind)) = matched {
            commands.push(Command {
                word_start: pos,
                word_end: pos + len,
                kind,
            });
            pos += len;
        } else {
            pos += 1;
        }
    }
    commands
}

/// True if some phrase in `table` matches starting exactly at `pos`.
fn starts_command_at(
    normalized: &[String],
    pos: usize,
    table: &[(&'static [&'static str], CommandKind)],
) -> bool {
    let n = normalized.len();
    for (phrase, kind) in table {
        if !is_command_phrase_kind(*kind) {
            // A following punctuation word doesn't count as "another
            // command" for the purposes of chaining (avoids "comma period"
            // ambiguity); only new-line/new-paragraph/scratch-that chain.
            continue;
        }
        let len = phrase.len();
        if pos + len > n {
            continue;
        }
        if (0..len).all(|k| normalized[pos + k] == phrase[k]) {
            return true;
        }
    }
    false
}

/// Renders `tokens` back into a string, applying `commands`.
fn render(tokens: &[Token<'_>], commands: &[Command]) -> String {
    let word_idx = word_indices(tokens);

    // Determine, for each command, the token-index deletion range for
    // ScratchThat (the preceding sentence/clause) up front, before mutating
    // anything, since deletion boundaries are computed against the
    // original token stream.
    let mut delete_token_ranges: Vec<(usize, usize)> = Vec::new();
    let mut punct_replacements: Vec<(usize, usize, &'static str)> = Vec::new(); // token range -> replacement
    let mut newline_ranges: Vec<(usize, usize, &'static str)> = Vec::new();

    for cmd in commands {
        let cmd_token_start = word_idx[cmd.word_start];
        let cmd_token_end_incl = word_idx[cmd.word_end - 1];

        match cmd.kind {
            CommandKind::ScratchThat => {
                let delete_start = find_clause_start(tokens, cmd_token_start);
                delete_token_ranges.push((delete_start, cmd_token_end_incl + 1));
            }
            CommandKind::NewLine => {
                newline_ranges.push((cmd_token_start, cmd_token_end_incl + 1, "\n"));
            }
            CommandKind::NewParagraph => {
                newline_ranges.push((cmd_token_start, cmd_token_end_incl + 1, "\n\n"));
            }
            CommandKind::Punct(sym) => {
                punct_replacements.push((cmd_token_start, cmd_token_end_incl + 1, sym));
            }
        }
    }

    // Merge all ranges (deletions, newlines, punctuation) into one list of
    // (start, end, replacement) ordered by start, then rebuild the string.
    // Later ScratchThat ranges may overlap earlier already-consumed text if
    // commands are adjacent; since commands are parsed left-to-right without
    // overlapping word usage, and clause boundaries only look backwards past
    // already-settled text, ranges here don't overlap each other.
    let mut edits: Vec<(usize, usize, &str)> = Vec::new();
    for (s, e) in delete_token_ranges {
        edits.push((s, e, ""));
    }
    for (s, e, sym) in newline_ranges {
        edits.push((s, e, sym));
    }
    for (s, e, sym) in punct_replacements {
        edits.push((s, e, sym));
    }
    edits.sort_by_key(|(s, _, _)| *s);

    let mut result = String::new();
    let mut cursor = 0usize; // token index
    for (start, end, replacement) in edits {
        let start = start.max(cursor);
        if start > cursor {
            for t in &tokens[cursor..start] {
                result.push_str(t.text);
            }
        }
        if end > start {
            result.push_str(replacement);
        }
        cursor = end.max(cursor);
    }
    for t in &tokens[cursor..] {
        result.push_str(t.text);
    }

    cleanup_whitespace(&result)
}

/// Finds the token index to delete from (inclusive) for a ScratchThat
/// command whose phrase starts at `cmd_token_start`. Walks backward over the
/// tokens preceding the command, stopping right after the most recent
/// sentence-ending punctuation (`.`, `!`, `?`) or clause separator (`,`), or
/// at the start of the text if none is found.
fn find_clause_start(tokens: &[Token<'_>], cmd_token_start: usize) -> usize {
    let mut i = cmd_token_start;
    while i > 0 {
        let prev = &tokens[i - 1];
        if prev.kind == TokenKind::Punct && matches!(prev.text, "." | "!" | "?" | ",") {
            return i; // start right after the separator (and any whitespace
                      // between it and the deleted clause is dropped by
                      // cleanup_whitespace).
        }
        i -= 1;
    }
    0
}

/// Collapses whitespace artifacts left behind by deletions/replacements:
/// multiple spaces, spaces before punctuation, leading space after a
/// newline, and trims leading/trailing whitespace. Does not touch
/// intentional blank lines (paragraph breaks).
fn cleanup_whitespace(s: &str) -> String {
    // Collapse runs of horizontal whitespace (space/tab) to a single space,
    // but preserve newlines as-is.
    let mut out = String::with_capacity(s.len());
    let mut last_was_space = false;
    for ch in s.chars() {
        if ch == '\n' {
            // Trim trailing space before a newline.
            while out.ends_with(' ') {
                out.pop();
            }
            out.push(ch);
            last_was_space = false;
        } else if ch == ' ' || ch == '\t' {
            if !last_was_space && !out.ends_with('\n') {
                out.push(' ');
            }
            last_was_space = true;
        } else {
            out.push(ch);
            last_was_space = false;
        }
    }

    // Remove a space that ends up right before punctuation, e.g. "hi , there".
    let mut cleaned = String::with_capacity(out.len());
    let chars: Vec<char> = out.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == ' ' && i + 1 < chars.len() && matches!(chars[i + 1], '.' | ',' | '!' | '?') {
            i += 1;
            continue;
        }
        cleaned.push(c);
        i += 1;
    }

    // Trim leading/trailing horizontal whitespace on each line, and overall.
    let trimmed_lines: Vec<&str> = cleaned.lines().map(|l| l.trim_matches(' ')).collect();
    let joined = trimmed_lines.join("\n");
    joined.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_commands_unchanged() {
        let text = "This is a normal sentence with no commands.";
        assert_eq!(apply_voice_edits(text), text);
    }

    #[test]
    fn no_commands_unchanged_spanish() {
        let text = "Este es un punto de vista normal sin comandos.";
        assert_eq!(apply_voice_edits(text), text);
    }

    #[test]
    fn scratch_that_deletes_preceding_clause() {
        // No punctuation anywhere before "scratch that", so the whole
        // utterance is one clause and gets dropped.
        let text = "I went to the store scratch that";
        assert_eq!(apply_voice_edits(text), "");
    }

    #[test]
    fn delete_that_deletes_preceding_clause() {
        let text = "I went to the store delete that";
        assert_eq!(apply_voice_edits(text), "");
    }

    #[test]
    fn borra_eso_deletes_preceding_clause() {
        let text = "Fui a la tienda borra eso";
        assert_eq!(apply_voice_edits(text), "");
    }

    #[test]
    fn scratch_that_stops_at_sentence_boundary() {
        let text = "I like cats. I went to the store scratch that";
        assert_eq!(apply_voice_edits(text), "I like cats.");
    }

    #[test]
    fn scratch_that_stops_at_comma() {
        let text = "First, I went to the store scratch that";
        assert_eq!(apply_voice_edits(text), "First,");
    }

    #[test]
    fn scratch_that_at_start_deletes_everything_before() {
        let text = "store scratch that";
        assert_eq!(apply_voice_edits(text), "");
    }

    #[test]
    fn new_line_inserts_newline() {
        let text = "first line new line second line";
        assert_eq!(apply_voice_edits(text), "first line\nsecond line");
    }

    #[test]
    fn nueva_linea_inserts_newline() {
        let text = "primera linea nueva linea segunda linea";
        assert_eq!(apply_voice_edits(text), "primera linea\nsegunda linea");
    }

    #[test]
    fn nueva_linea_with_accent_inserts_newline() {
        let text = "primera nueva línea segunda";
        assert_eq!(apply_voice_edits(text), "primera\nsegunda");
    }

    #[test]
    fn new_paragraph_inserts_blank_line() {
        let text = "first paragraph new paragraph second paragraph";
        assert_eq!(
            apply_voice_edits(text),
            "first paragraph\n\nsecond paragraph"
        );
    }

    #[test]
    fn nuevo_parrafo_with_accent_inserts_blank_line() {
        let text = "primero nuevo párrafo segundo";
        assert_eq!(apply_voice_edits(text), "primero\n\nsegundo");
    }

    #[test]
    fn spoken_comma_at_end_of_utterance() {
        let text = "please pick up milk comma";
        assert_eq!(apply_voice_edits(text), "please pick up milk,");
    }

    #[test]
    fn spoken_period_at_end_of_utterance() {
        let text = "that is all period";
        assert_eq!(apply_voice_edits(text), "that is all.");
    }

    #[test]
    fn spoken_coma_at_end_of_utterance_spanish() {
        let text = "compra leche coma";
        assert_eq!(apply_voice_edits(text), "compra leche,");
    }

    #[test]
    fn spoken_punto_at_end_of_utterance_spanish() {
        let text = "eso es todo punto";
        assert_eq!(apply_voice_edits(text), "eso es todo.");
    }

    #[test]
    fn question_mark_at_end_of_utterance() {
        let text = "are you coming question mark";
        assert_eq!(apply_voice_edits(text), "are you coming?");
    }

    #[test]
    fn signo_de_interrogacion_at_end_spanish() {
        let text = "vienes signo de interrogacion";
        assert_eq!(apply_voice_edits(text), "vienes?");
    }

    #[test]
    fn punto_de_vista_not_treated_as_punctuation() {
        // "punto" here is followed by ordinary words, not end-of-utterance
        // or another command, so it must stay a literal word.
        let text = "ese es mi punto de vista";
        assert_eq!(apply_voice_edits(text), text);
    }

    #[test]
    fn point_of_view_not_treated_as_punctuation() {
        let text = "that is my point of view";
        assert_eq!(apply_voice_edits(text), text);
    }

    #[test]
    fn punto_followed_by_new_line_is_punctuation() {
        let text = "eso es todo punto new line siguiente";
        assert_eq!(apply_voice_edits(text), "eso es todo.\nsiguiente");
    }

    #[test]
    fn comma_mid_utterance_followed_by_words_is_not_punctuation() {
        // "comma" mid-sentence followed by ordinary words is left alone,
        // since it is neither at the end nor followed by a command phrase.
        let text = "the comma splice is a common error";
        assert_eq!(apply_voice_edits(text), text);
    }

    #[test]
    fn idempotent_scratch_that() {
        let text = "I like cats. I went to the store scratch that";
        let once = apply_voice_edits(text);
        let twice = apply_voice_edits(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn idempotent_new_paragraph() {
        let text = "first paragraph new paragraph second paragraph";
        let once = apply_voice_edits(text);
        let twice = apply_voice_edits(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn idempotent_punctuation() {
        let text = "that is all period";
        let once = apply_voice_edits(text);
        let twice = apply_voice_edits(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn idempotent_plain_text() {
        let text = "nothing special here at all";
        let once = apply_voice_edits(text);
        let twice = apply_voice_edits(&once);
        assert_eq!(once, once.clone());
        assert_eq!(twice, text);
    }

    #[test]
    fn empty_string_unchanged() {
        assert_eq!(apply_voice_edits(""), "");
    }

    #[test]
    fn case_insensitive_matching() {
        let text = "I went to the store SCRATCH THAT";
        assert_eq!(apply_voice_edits(text), "");
    }

    #[test]
    fn multiple_commands_in_one_utterance() {
        // "comma" here is immediately followed by "new line", a recognized
        // command phrase, so it is treated as punctuation per the rule.
        let text = "buy milk comma new line buy eggs";
        assert_eq!(apply_voice_edits(text), "buy milk,\nbuy eggs");
    }
}
