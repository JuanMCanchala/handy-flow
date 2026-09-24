# handy-flow roadmap

Goal: a Wispr Flow–style dictation app (with voice commands) on top of Handy's Rust/Tauri core.

| #   | Item                                                                                                   | Status      |
| --- | ------------------------------------------------------------------------------------------------------ | ----------- |
| 0   | Build Handy as-is on Windows, measure RAM                                                              | in progress |
| 1   | Command mode: dedicated hotkey + "Hey <agent>", LLM rewrites selected text in place                    | todo        |
| 2   | Cloud transcription provider (OpenAI-compatible: Fireworks, Groq, OpenAI)                              | todo        |
| 3   | Style per app (cleanup tone based on the active app)                                                   | todo        |
| 4   | Snippets (trigger phrase → expansion)                                                                  | todo        |
| 5   | Transforms (saved prompts applied to the selection)                                                    | todo        |
| 6   | Insights (total words, wpm, day streak from history)                                                   | todo        |
| 7   | Flow-style Home (history grouped by day, play / copy)                                                  | todo        |
| 8   | Scratchpad                                                                                             | todo        |
| 9   | Live translation EN ↔ ES (Transync-style live subtitles + dictate in one language, paste in the other) | todo        |

## Windows build prerequisites

- Visual Studio with C++ tools, CMake (`winget install Kitware.CMake`), Vulkan SDK (`winget install KhronosGroup.VulkanSDK`)
- `src-tauri/resources/models/silero_vad_v4.onnx` from `https://blob.handy.computer/silero_vad_v4.onnx`

## Live translation EN ↔ ES (item 9) — design

Reference product: Transync AI (bilingual split-screen live interpretation, $8.99/mo for 10 h).

- **Mode A — dictate in one language, paste in the other**: translate prompt through the existing LLM post-processing client (Fireworks/Groq). Cheapest and highest value.
- **Mode B — live bilingual subtitles**: always-on-top overlay, source line + translated line. VAD-segmented ASR partials (local Parakeet/Whisper), translate only finals with the last 2–3 finals as context.
- `Translator` trait on the Rust side: `LlmTranslator` (existing client) first, `Ct2Translator` (ct2rs + Opus-MT en↔es, ~300 MB, offline) later.
- Optional later: OpenAI `gpt-realtime-translate` for spoken output.
