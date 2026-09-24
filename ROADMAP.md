# handy-flow roadmap

Goal: a Wispr Flow–style dictation app (with voice commands) on top of Handy's Rust/Tauri core.

| #   | Item                                                                                                   | Status      |
| --- | ------------------------------------------------------------------------------------------------------ | ----------- |
| 0   | Build Handy as-is on Windows, measure RAM                                                              | in progress |
| 1   | Command mode: dedicated hotkey + "Hey <agent>", LLM rewrites selected text in place                    | done        |
| 2   | Cloud transcription provider (OpenAI-compatible: Fireworks, Groq, OpenAI)                              | todo        |
| 3   | Style per app (cleanup tone based on the active app)                                                   | todo        |
| 4   | Snippets (trigger phrase → expansion)                                                                  | todo        |
| 5   | Transforms (saved prompts applied to the selection)                                                    | todo        |
| 6   | Insights (total words, wpm, day streak from history)                                                   | todo        |
| 7   | Flow-style Home (history grouped by day, play / copy)                                                  | todo        |
| 8   | Scratchpad                                                                                             | todo        |
| 9   | Live translation EN ↔ ES (Transync-style live subtitles + dictate in one language, paste in the other) | todo        |
| 10  | Profile copilot: load my profile, listen to questions about me, suggest an answer in EN or ES         | todo        |

## Windows build prerequisites

- Visual Studio with C++ tools, CMake (`winget install Kitware.CMake`), Vulkan SDK (`winget install KhronosGroup.VulkanSDK`)
- `src-tauri/resources/models/silero_vad_v4.onnx` from `https://blob.handy.computer/silero_vad_v4.onnx`

## Live translation EN ↔ ES (item 9) — design

Reference product: Transync AI (bilingual split-screen live interpretation, $8.99/mo for 10 h).

- **Mode A — dictate in one language, paste in the other**: translate prompt through the existing LLM post-processing client (Fireworks/Groq). Cheapest and highest value.
- **Mode B — live bilingual subtitles**: always-on-top overlay, source line + translated line. VAD-segmented ASR partials (local Parakeet/Whisper), translate only finals with the last 2–3 finals as context.
- `Translator` trait on the Rust side: `LlmTranslator` (existing client) first, `Ct2Translator` (ct2rs + Opus-MT en↔es, ~300 MB, offline) later.
- Optional later: OpenAI `gpt-realtime-translate` for spoken output.

Code to borrow (both MIT):
- [NBS282/LiveTranslate](https://github.com/NBS282/LiveTranslate) — Tauri 2 + transcribe-cpp + MarianMT EN↔ES overlay: `translation/segmenter.rs` (VAD sentence segmentation), `translation/live.rs` (partial/commit), transparent always-on-top overlay config.
- [wxkingstar/TransEcho](https://github.com/wxkingstar/TransEcho) — WASAPI loopback system-audio capture via cpal (`audio/capture_windows.rs`, `resample.rs`) to subtitle the other side of a call.
- Reference only: SakiRinn/LiveCaptions-Translator (Apache-2.0, context-window prompting), ufal/whisper_streaming (LocalAgreement commit policy). kizuna-ai-lab/sokuji is AGPL — ideas only.

## Profile copilot (item 10) — design

Load my profile (CV, LinkedIn text, notes), listen to the call, detect questions asked to me, suggest a first-person, speakable answer in EN, ES or both.

- Borrow from [naxhq/NexQ](https://github.com/naxhq/NexQ) (Tauri 2 + Rust, MIT): `intelligence/question_detector.rs` (add Spanish cues: "cuéntame", "háblame de", "¿cómo…"), `rag/` + `context/` loaders (PDF/DOCX/MD), `intelligence/prompt_templates.rs`, `audio/system_capture.rs` (wasapi + cpal loopback), `commands/stealth_commands.rs` (overlay hidden from screen share).
- Prompt style reference: JWM0203/MeetingCopilot (Apache-2.0, first-person teleprompter answers). Echo cancellation reference: Laxcorp-Research/project-raven (MIT, WebRTC AEC3).
- GPL-3.0 (pluely, cheating-daddy, glass) are ideas only; Natively is non-commercial — take nothing.
- Pipeline shared with item 9: mic ("You") + system audio ("Them") → local ASR on "Them" → question detector (rules, optional small-LLM check) after a short pause → profile in prompt (RAG only for long notes) → streamed answer card in the same hidden overlay as the live subtitles.
