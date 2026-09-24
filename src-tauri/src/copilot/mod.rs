//! Profile copilot: the user pastes/imports profile text (CV, LinkedIn
//! summary, notes); while listening (piggy-backing on the `live_translate`
//! capture/VAD/transcription pipeline in copilot mode), closed transcript
//! segments are checked for questions (`question_detector`), and detected
//! questions are answered by the configured LLM grounded in the profile
//! (`prompt`), then surfaced as answer cards and history entries.

mod history;
mod import;
mod profile;
mod prompt;
mod question_detector;

pub use history::{append_entry, clear_history, get_history, CopilotAnswerEntry};
pub use import::extract_text;
pub use profile::{get_profile, write_profile, CopilotAnswerLanguage, CopilotProfile};
pub use prompt::build_answer_prompt;
pub use question_detector::is_question;
