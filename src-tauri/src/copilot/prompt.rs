//! Builds the answer-suggestion prompt sent to the configured LLM
//! (`settings::resolve_llm_target` + `llm_client::send_chat_completion`).
//!
//! Prompt style adapted from the interview-answer prompt approach in NexQ
//! (https://github.com/naxhq/NexQ, MIT licensed): first person, concise,
//! speakable, and grounded strictly in the supplied profile text — the model
//! is told to say what's missing rather than invent facts.

use super::profile::CopilotAnswerLanguage;

/// Max number of recent transcript segments included as conversation context.
pub const MAX_TRANSCRIPT_CONTEXT: usize = 10;

/// System + user messages for one answer suggestion.
pub struct AnswerPrompt {
    /// Instructions and the profile. Identical for every question in a
    /// session, so providers with prefix caching (Fireworks, Groq, OpenAI)
    /// skip re-reading the CV and the first word arrives sooner.
    pub system: String,
    /// Recent conversation plus the question just asked.
    pub user: String,
}

/// Builds the prompt for one answer suggestion.
///
/// `profile` is the user's raw CV/notes text, `recent_transcript` is the last
/// `MAX_TRANSCRIPT_CONTEXT` (or fewer) transcript segments in speaking order,
/// and `question` is the just-closed utterance that triggered detection.
pub fn build_answer_prompt(
    profile: &str,
    recent_transcript: &[String],
    question: &str,
    language: CopilotAnswerLanguage,
) -> AnswerPrompt {
    let mut system = String::new();

    system.push_str(
        "You are silently listening to a call/interview on behalf of the user and \
must suggest how THEY should answer the question just asked to them.\n\n",
    );

    system.push_str(
        "Write the answer the user should say out loud, in first person (\"I\", \"my\"), \
2 to 4 short sentences, natural and speakable — not a bullet list. Lead with the direct \
answer in the first sentence, then one concrete detail (project, tool, number) from the \
profile. Only use facts present in the profile or conversation. Stay consistent with \
what the user (\"Me\") already said in the conversation: build on it, do not repeat it and \
never contradict it; for a follow-up question, continue the thread of their previous answer. \
If the profile does not \
contain what is needed, give an honest, general answer the user can adapt and never invent \
employers, dates or numbers.\n",
    );

    match language {
        CopilotAnswerLanguage::Auto => {
            system.push_str(
                "Answer in the same language as the question. When answering in English, \
use simple, clear English (B1-B2 level, common words, short sentences) that a non-native \
speaker can read aloud fluently.\n",
            );
        }
        CopilotAnswerLanguage::En => {
            system.push_str(
                "Answer in simple, clear English (B1-B2 level, common words, short \
sentences) that a non-native speaker can read aloud fluently.\n",
            );
        }
        CopilotAnswerLanguage::Es => {
            system.push_str("Answer in Spanish.\n");
        }
        CopilotAnswerLanguage::Both => {
            system.push_str(
                "Answer in simple, clear English (B1-B2 level, common words, short \
sentences) that a non-native speaker can read aloud fluently, then give its Spanish \
translation so the user knows exactly what they are saying. Reply with exactly two labeled \
paragraphs, English first, in this format:\nEN: <english answer>\nES: <spanish translation \
of that answer>\n",
            );
        }
    }
    system.push_str("Reply with only the answer (or the EN/ES pair above), no preamble.\n\n");

    system.push_str("User profile (CV / notes, ground truth about the user):\n");
    if profile.trim().is_empty() {
        system.push_str("(no profile provided)\n");
    } else {
        system.push_str(profile.trim());
        system.push('\n');
    }

    let mut user = String::new();
    if !recent_transcript.is_empty() {
        user.push_str(
            "Recent conversation (oldest first, for context only). \"Interviewer\" is the \
other side of the call; \"Me\" is the user, whose answers you are helping with:\n",
        );
        let skip = recent_transcript.len().saturating_sub(MAX_TRANSCRIPT_CONTEXT);
        for line in &recent_transcript[skip..] {
            user.push_str("- ");
            user.push_str(line);
            user.push('\n');
        }
        user.push('\n');
    }
    user.push_str("Question just asked to the user:\n\"");
    user.push_str(question.trim());
    user.push('"');

    AnswerPrompt { system, user }
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
        assert!(prompt.system.contains("5 years of Rust experience."));
        assert!(prompt.user.contains("Why do you want this job?"));
        assert!(prompt.system.contains("same language as the question"));
    }

    #[test]
    fn empty_profile_tells_the_model_nothing_was_provided() {
        let prompt = build_answer_prompt("", &[], "What is your name?", CopilotAnswerLanguage::En);
        assert!(prompt.system.contains("(no profile provided)"));
        assert!(prompt.system.contains("simple, clear English"));
    }

    #[test]
    fn both_language_asks_for_labeled_en_es_pair() {
        let prompt = build_answer_prompt("Profile", &[], "Question?", CopilotAnswerLanguage::Both);
        assert!(prompt.system.contains("EN:"));
        assert!(prompt.system.contains("ES:"));
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
        let idx_first = prompt.user.find("Hi there.").unwrap();
        let idx_second = prompt.user.find("Thanks for joining.").unwrap();
        let idx_question = prompt.user.find("Question just asked").unwrap();
        assert!(idx_first < idx_second);
        assert!(idx_second < idx_question);
    }

    #[test]
    fn conversation_keeps_speaker_roles_and_asks_to_follow_the_user() {
        let context = vec![
            "Interviewer: Tell me about SIVA.".to_string(),
            "Me: I built the money flows.".to_string(),
        ];
        let prompt = build_answer_prompt("Profile", &context, "How?", CopilotAnswerLanguage::En);
        assert!(prompt.user.contains("- Me: I built the money flows."));
        assert!(prompt.user.contains("\"Me\" is the user"));
        assert!(prompt.system.contains("continue the thread"));
    }

    #[test]
    fn only_up_to_max_context_segments_are_included() {
        let context: Vec<String> = (0..14).map(|i| format!("segment {i}.")).collect();
        let prompt = build_answer_prompt("Profile", &context, "Q?", CopilotAnswerLanguage::Auto);
        // The newest lines are kept, the oldest dropped.
        assert!(!prompt.user.contains("segment 3."));
        assert!(prompt.user.contains("segment 4."));
        assert!(prompt.user.contains("segment 13."));
    }
}
