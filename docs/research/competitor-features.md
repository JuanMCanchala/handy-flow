# Competitor feature research — handy-flow

**Method note:** no web-search/fetch tool was available in this run. Everything below comes from
model knowledge (training data, not live pages). URLs are given for verification, but **pricing is
approximate and must be re-checked against the vendor page before it is quoted anywhere.**

**Legend:** `[covered]` = already in the handy-flow plan (push-to-talk + AI cleanup, "Hey Flow"
command mode on selection, cloud/local transcription, style per app, snippets, transforms, custom
dictionary, insights, history home, scratchpad, live EN↔ES translation with bilingual subtitles,
profile copilot).

---

## 1. Per-app summary

### Granola — https://granola.ai

- Captures system audio of any meeting **without a bot joining**; notes are generated from the
  transcript + the sparse notes you typed yourself.
- Template-driven AI notes (standup, 1:1, sales discovery, user interview) applied after the call.
- "Chat with your meetings": ask questions across a single meeting or the whole workspace.
- Auto-enhances your raw scratch notes instead of producing a generic summary — the differentiator.
- Shared folders / team workspaces with permissioned meeting collections.
- Pricing: free tier limited to a small number of meetings, individual ~$18/user/mo, business tier above that.

### Wispr Flow — https://wisprflow.ai

- Push-to-talk dictation anywhere with LLM cleanup (filler removal, punctuation, tone). `[covered]`
- Context awareness: reads the active app / focused field / selected text to adapt formatting. Partly `[covered]` (style per app).
- Whispering mode + auto-detected code-switching between languages in a single utterance.
- Command mode: speak an instruction to edit selected text. `[covered]`
- Personal dictionary that self-learns from your edits; snippets. Partly `[covered]`.
- Mobile keyboard companion (iOS) sharing the same dictionary/history.
- Pricing: free weekly word quota (~2k words/week), Pro ~$12–15/mo, Teams per-seat.

### Superwhisper — https://superwhisper.com

- **Modes**: named presets each with its own model, prompt, language and output format (email, note, code, custom).
- Fully local Whisper/Parakeet inference with optional cloud models; offline by default.
- Drag-and-drop **audio/video file transcription** with SRT/VTT export.
- Custom vocabulary + replacements, and per-mode LLM post-processing with variables (clipboard, selected text, app name).
- Voice-triggered workflows (Shortcuts / URL schemes) so dictation can drive automation.
- Pricing: free limited tier, Pro ~$8.49/mo, lifetime license ~$250.

### MacWhisper — https://goodsnooze.gumroad.com/l/macwhisper

- Batch/bulk transcription of local media files, plus podcast/YouTube URL ingestion.
- Full transcript editor with timestamps, speaker labels, search, and SRT/VTT/CSV export.
- System-audio capture to transcribe meetings/videos playing on the machine.
- Dictation mode with local models; AI summarize/ask over a finished transcript.
- Pricing: free basic, Pro one-time purchase ~€45–60 (no subscription).

### VoiceInk — https://tryvoiceink.com

- Open-source (GPL) macOS dictation with local Whisper/Parakeet; direct comparison target for us.
- **Power mode**: auto-switches model/prompt/language based on the frontmost app or URL.
- AI enhancement with user-editable prompt templates and context injection (screen text, clipboard).
- Local transcript history with search; bring-your-own cloud API key.
- Pricing: one-time license ~$29 (free trial), source buildable for free.

### Otter.ai — https://otter.ai

- Real-time streaming transcript with live word-by-word display and speaker diarization.
- Auto-join bot for Zoom/Meet/Teams from the connected calendar.
- "Otter AI Chat" + automatic action-item extraction and channel sharing.
- Collaborative transcript: comments, highlights, snippet clips shared by link.
- Pricing: free ~300 min/mo, Pro ~$17/mo, Business ~$30/user/mo.

### Fireflies.ai — https://fireflies.ai

- Notetaker bot on all major conferencing platforms + calendar auto-scheduling.
- **CRM/tool sync** (Salesforce, HubSpot, Slack, Notion) pushing notes and fields automatically.
- Conversation intelligence: talk-time ratio, sentiment, monologue length, topic trackers.
- "AskFred" chat over one meeting or the whole searchable meeting corpus.
- Pricing: free limited, Pro ~$18/user/mo, Business ~$29, Enterprise ~$39.

### Fathom — https://fathom.video

- Free unlimited recording/transcription — the aggressive freemium anchor in this space.
- One-click, timestamped **live highlights** during the call; summary ready seconds after it ends.
- Multiple summary templates per meeting type; auto-sync to CRM with follow-up email drafts.
- Ask Fathom across all past meetings; team "Ask" for shared knowledge.
- Pricing: free unlimited core, Premium ~$19/user/mo, Team ~$29–39/user/mo.

### tl;dv — https://tldv.io

- Recording + transcription in 30+ languages with timestamped comments and reactions.
- **Multi-meeting AI reports** ("what did all customers say about pricing this month") on a schedule.
- Clip/reel creation from moments, shareable to Slack/Notion/docs.
- Speaker-level coaching metrics and sales-playbook scorecards.
- Pricing: generous free tier, Pro ~$18–29/user/mo, Business above.

### Krisp — https://krisp.ai

- Real-time **AI noise + background-voice cancellation** as a virtual mic/speaker device.
- Echo cancellation and "voice clarity"/accent-conversion processing on-device.
- Bot-free meeting recording and transcription layered on the audio device.
- Live talk-time/monologue analytics and meeting notes.
- Pricing: free with daily noise-cancel minutes, Pro ~$8/mo, Business ~$15/user/mo.

### Cluely — https://cluely.com

- Real-time on-screen assistant that listens to the call and suggests answers live. `[covered]` (profile copilot).
- Reads the screen for context (docs, job description, code) to ground suggestions.
- Overlay designed to be **invisible to screen sharing / recording**.
- Post-session summary of the conversation and of what was suggested.
- Pricing: free limited, Pro ~$20/mo, Enterprise custom.

### Raycast AI — https://raycast.com/ai

- AI Commands: user-defined prompts bound to hotkeys, taking selected text/clipboard as input. Partly `[covered]` (transforms).
- Model choice across providers (GPT/Claude/Gemini/local) inside one UI, with per-command model.
- AI Chat with tools/extensions that can act on apps (create issue, search web, run script).
- Dictation-to-anywhere and "AI Presets"; deep-links so other apps can invoke commands.
- Pricing: app free, Pro ~$8–10/mo, Advanced AI add-on ~$10/mo.

### Notion AI meeting notes — https://notion.com/product/ai

- Records and transcribes in-page; notes land as native Notion blocks in the workspace.
- Speaker labels + AI summary and action items written directly into the doc.
- AI search/chat grounded in the whole workspace (docs + meetings + connected Slack/Drive).
- Database properties auto-filled from the transcript (AI autofill) for pipelines/CRMs.
- Pricing: bundled into Business/Enterprise plans (~$20–25/user/mo); no standalone AI seat.

### Monologue — https://monologue.to

- Minimal always-listening dictation bar for macOS; hotkey-free flow with VAD start/stop.
- LLM formatting tuned per target app, including code-aware output in editors.
- Voice commands to control the host app (open file, run action) rather than only insert text.
- Emphasis on very low latency for short utterances.
- Pricing: small monthly subscription (~$10/mo) with free trial; verify.

### Aqua Voice — https://withaqua.com

- Dictation treated as **editing**: "scratch that", "make that a bullet list", inline self-corrections.
- Learns formatting and vocabulary from your accepted/rejected outputs over time.
- Context from the screen + selected text; strong on technical/medical jargon.
- Very low-latency streaming with partial text shown before finalization.
- Pricing: free tier, Pro ~$10–12/mo, team plans.

### Willow Voice — https://willowvoice.com

- Dictation with automatic tone/format adaptation per app (Slack vs email vs doc). `[covered]`
- Self-learning dictionary from corrections; no manual entry needed for names.
- Auto-removal of self-corrections and stutters from the final text.
- Privacy posture: no audio retention by default; SOC2 messaging for teams.
- Pricing: free trial, ~$12/mo Pro, team seats.

---

## 2. Gap table — features we do **not** have yet

| Feature | Apps that have it | User value | Effort | Local-first feasible |
| --- | --- | --- | --- | --- |
| System-audio (loopback) capture for meetings | Granola, MacWhisper, Krisp, Notion | H | M | yes |
| Speaker diarization / speaker labels | Otter, Fireflies, MacWhisper, Notion | H | L | yes (pyannote/ONNX) |
| Streaming partial transcript while speaking | Otter, Aqua, Monologue | H | M | yes |
| Audio/video **file import** + batch transcription | MacWhisper, Superwhisper | H | S | yes |
| SRT/VTT/timestamped export | MacWhisper, Superwhisper, tl;dv | M | S | yes |
| Named **modes/presets** (model+prompt+language per mode) | Superwhisper, VoiceInk, Raycast | H | S | yes |
| Inline voice editing commands ("scratch that", "bullet list") | Aqua, Superwhisper | H | M | yes |
| Screen-text context injection into the LLM prompt | VoiceInk, Cluely, Aqua | M | M | yes (OS accessibility APIs) |
| Self-learning dictionary from user corrections | Wispr, Willow, Aqua | H | M | yes |
| Automatic multilingual code-switching in one utterance | Wispr | M | S | yes (Whisper multilingual) |
| Chat/ask over transcript history (semantic search + RAG) | Granola, Fireflies, Fathom, tl;dv | H | M | partly (local embeddings yes, good answers want LLM) |
| Meeting-note templates (standup/1:1/interview) | Granola, Fathom, tl;dv | M | S | yes |
| Action-item / task extraction | Otter, Fireflies, Notion | M | S | yes |
| Live in-call highlight/bookmark key | Fathom, tl;dv | M | S | yes |
| Calendar integration + auto-record scheduled calls | Otter, Fireflies, Granola | M | L | no (needs OAuth to cloud calendars) |
| Conferencing **bot** that joins remotely | Otter, Fireflies, tl;dv | L | L | no |
| Noise / background-voice suppression on input | Krisp | M | M | yes (RNNoise/DeepFilterNet ONNX) |
| Overlay hidden from screen capture | Cluely | L | M | yes (platform window flags) |
| Voice-driven app automation (run action, not just insert text) | Raycast, Monologue, Superwhisper | M | M | yes |
| Third-party sync (Notion/Slack/CRM/webhook out) | Fireflies, Fathom, tl;dv, Notion | M | S (webhook) / L (per-integration) | partly |
| Shareable transcript links / collaboration | Otter, tl;dv, Granola | L | L | no |
| Talk-time / sentiment conversation analytics | Fireflies, Krisp, tl;dv | L | M | yes |
| Mobile companion keyboard | Wispr | M | L | no |
| Undo / retry last dictation with another model | Superwhisper (partial) | M | S | yes |
| Model-race: fast local draft, cloud refine | none seen (differentiator) | M | M | yes |

---

## 3. Top 10 recommendations (ranked by value ÷ effort)

1. **Named modes/presets** — generalize the existing per-app style into a `Mode` record
   (model id, prompt, target language, output format, optional hotkey) in `settings.rs`, resolved at
   transcription start in `managers/transcription.rs`. Frontend: a modes list under
   `components/settings/` reusing the model-selector picker. Per-app style becomes a mode binding.
2. **Audio/video file import + batch transcription** — a Tauri command that takes file paths, decodes
   with the existing resampler in `audio_toolkit/audio/`, and feeds the same pipeline with a job queue.
   Reuses everything; UI is a drop zone on the history home that writes normal history rows.
3. **SRT/VTT export with timestamps** — persist per-segment timestamps already returned by
   transcribe-cpp/transcribe-rs into a `segments` table beside history, then a pure-Rust formatter
   command `export_transcript(id, format)`. Unlocks the file-import use case and costs little.
4. **Streaming partial transcript** — run the VAD-chunked buffer through the small local model on a
   rolling window, emitting `transcription://partial` events to the overlay React app; final pass
   re-transcribes the full utterance so quality is unchanged. Biggest perceived-latency win.
5. **Inline voice editing commands** — post-pass in the LLM cleanup step: give the client the raw
   text plus a rule list ("scratch that", "new paragraph", "make that a list") so edits are resolved
   by the same OpenAI-compatible call already used for cleanup. Optional deterministic pre-filter in
   Rust for the 5 most common phrases so it works with no LLM configured.
6. **Self-learning dictionary** — when the user edits a transcript in the history UI, diff original vs
   edited in Rust, store candidate term pairs in SQLite with a hit counter, and promote terms past a
   threshold into the existing custom dictionary (surfacing them for one-click confirm).
7. **System-audio capture** — add a loopback source to `audio_toolkit/audio/` (WASAPI loopback on
   Windows, ScreenCaptureKit on macOS, PulseAudio monitor on Linux) as a second input stream mixed or
   kept separate from the mic. Gates recommendations 8 and 9; do it behind a debug-mode flag first.
8. **Speaker diarization on long recordings** — an ONNX segmentation+embedding model run via the
   existing ONNX runtime used by transcribe-rs, clustering embeddings per VAD segment and labelling
   segments before they hit SQLite. Only applied to file-import/system-audio sessions, not dictation.
9. **Note templates + action items** — a template = a stored prompt applied to a whole session
   transcript (reuse the transforms plumbing, just scoped to session instead of clipboard). Render the
   result as markdown in the history detail view; extract `- [ ]` items into a checklist field.
10. **Ask-your-history (local RAG)** — embed each history row with a small local embedding model,
    store vectors in SQLite (BLOB + brute-force cosine is enough at this corpus size), and expose a
    search/chat panel on the history home that stuffs top-k rows into the OpenAI-compatible client.

**Deliberately deferred:** conferencing bots, calendar OAuth, shared links, mobile keyboard — all
require cloud infrastructure and contradict the local-first positioning.
