//! Builds the answer-suggestion prompt sent to the configured LLM
//! (`settings::resolve_llm_target` + `llm_client::send_chat_completion`).
//!
//! Prompt style adapted from the interview-answer prompt approach in NexQ
//! (https://github.com/naxhq/NexQ, MIT licensed): first person, concise,
//! speakable, and grounded strictly in the supplied profile text — the model
//! is told to say what's missing rather than invent facts.

use super::profile::CopilotAnswerLanguage;

/// Max number of recent transcript segments included as conversation context.
pub const MAX_TRANSCRIPT_CONTEXT: usize = 6;

/// Builds the user prompt for one answer suggestion.
///
/// `profile` is the user's raw CV/notes text, `recent_transcript` is the last
/// `MAX_TRANSCRIPT_CONTEXT` (or fewer) transcript segments in speaking order,
/// and `question` is the just-closed segment that triggered detection.
pub fn build_answer_prompt(
    profile: &str,
    recent_transcript: &[String],
    question: &str,
    language: CopilotAnswerLanguage,
) -> String {
    let mut prompt = String::new();

    prompt.push_str(
        "You are silently listening to a call/interview on behalf of the user and \
must suggest how THEY should answer a question just asked to them.\n\n",
    );

    prompt.push_str("User profile (CV / notes, ground truth about the user):\n");
    if profile.trim().is_empty() {
        prompt.push_str("(no profile provided)\n");
    } else {
        prompt.push_str(profile.trim());
        prompt.push('\n');
    }

    if !recent_transcript.is_empty() {
        prompt.push_str("\nRecent conversation (oldest first, for context only):\n");
        for line in recent_transcript.iter().take(MAX_TRANSCRIPT_CONTEXT) {
            prompt.push_str("- \"");
            prompt.push_str(line);
            prompt.push_str("\"\n");
        }
    }

    prompt.push_str("\nQuestion just asked to the user:\n\"");
    prompt.push_str(question.trim());
    prompt.push_str("\"\n\n");

    prompt.push_str(
        "Write the answer the user should say out loud, in first person (\"I\", \"my\"), \
2 to 4 sentences, natural and speakable — not a bullet list. \
Only use facts present in the profile or conversation above. \
If the profile does not contain what is needed to answer, say so plainly \
(e.g. mention that detail isn't in the profile) instead of inventing facts.\n",
    );

    match language {
        CopilotAnswerLanguage::Auto => {
            prompt.push_str("Answer in the same language as the question above.\n");
        }
        CopilotAnswerLanguage::En => {
            prompt.push_str("Answer in English.\n");
        }
        CopilotAnswerLanguage::Es => {
            prompt.push_str("Answer in Spanish.\n");
        }
        CopilotAnswerLanguage::Both => {
            prompt.push_str(
                "Answer in both English and Spanish. Reply with exactly two labeled \
paragraphs, in this format:\nEN: <english answer>\nES: <spanish answer>\n",
            );
        }
    }

    prompt.push_str("\nReply with only the answer (or the EN/ES pair above), no preamble.");
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_includes_profile_question_and_language_instruction() {
        let prompt = build_answer_prompt(
            "5 years of Rust experience.",
            &[],
            "Why do you want this job?",
            CopilotAnswerLanguage::Auto,
        );
        assert!(prompt.contains("5 years of Rust experience."));
        assert!(prompt.contains("Why do you want this job?"));
        assert!(prompt.contains("same language as the question"));
    }

    #[test]
    fn empty_profile_tells_the_model_nothing_was_provided() {
        let prompt = build_answer_prompt("", &[], "What is your name?", CopilotAnswerLanguage::En);
        assert!(prompt.contains("(no profile provided)"));
        assert!(prompt.contains("Answer in English."));
    }

    #[test]
    fn both_language_asks_for_labeled_en_es_pair() {
        let prompt = build_answer_prompt("Profile", &[], "Question?", CopilotAnswerLanguage::Both);
        assert!(prompt.contains("EN:"));
        assert!(prompt.contains("ES:"));
    }

    #[test]
    fn recent_transcript_context_is_included_in_order() {
        let context = vec!["Hi there.".to_string(), "Thanks for joining.".to_string()];
        let prompt = build_answer_prompt(
            "Profile",
            &context,
            "How are you?",
            CopilotAnswerLanguage::Auto,
        );
        let idx_first = prompt.find("Hi there.").unwrap();
        let idx_second = prompt.find("Thanks for joining.").unwrap();
        let idx_question = prompt.find("Question just asked").unwrap();
        assert!(idx_first < idx_second);
        assert!(idx_second < idx_question);
    }

    #[test]
    fn only_up_to_max_context_segments_are_included() {
        let context: Vec<String> = (0..10).map(|i| format!("segment {i}")).collect();
        let prompt = build_answer_prompt("Profile", &context, "Q?", CopilotAnswerLanguage::Auto);
        assert!(!prompt.contains("segment 6"));
        assert!(prompt.contains("segment 5"));
    }
}
