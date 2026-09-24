# Performance: memory and bundle-size pass (Windows)

Measured with `scripts/measure-ram.ps1` against a release build
(`bun run tauri build --no-bundle`, `target-perf\release\handy.exe`) on
Windows 11. All numbers are the sum of the main `handy.exe` process plus its
own WebView2 helper processes only (filtered by the helpers'
`--webview-exe-name=handy.exe` command-line argument, which excludes
msedgewebview2 helpers spawned by unrelated apps like Windows
Search/Widgets — an early measurement pass that matched by process name
alone over-counted for this reason).

## Before / after: main window hidden to tray

| State | Processes | Working set | Private |
| --- | --- | --- | --- |
| Window open | 8 | 648.1 MB | 413.8 MB |
| Hidden to tray (before this change — `window.hide()` only) | 8 | ~648 MB (WebView2 stays fully resident; no measurement needed, code path was a no-op hide) | ~414 MB |
| **Hidden to tray (after — WebView2 destroyed)** | 8 | **609.2 MB** | **371.3 MB** |

The saving from destroying the main window's WebView2 on hide is real but
modest (~39 MB working set / ~43 MB private on this machine), because the
main window's own renderer process is only one of several WebView2
processes the app keeps resident. See "What we found" below for why the
saving isn't larger, and why we didn't chase it further.

Model-loaded states (per the brief: "after one dictation with a local model
loaded (Parakeet/Canary if present)" and "60s after the model unload
timeout") were **not measured**: this checkout's
`src-tauri/resources/models/` only contains the Silero VAD model
(`silero_vad_v4.onnx`) and a vocab file, no Whisper/Parakeet/Canary weights.
Downloading one (487 MB–1.6 GB) was out of scope for this pass. The model
unload path itself (`managers/transcription.rs`, idle-watcher thread
checking every 10s against `settings.model_unload_timeout`, default 5 min)
was not modified — it already existed and already frees the loaded model's
buffers on timeout.

Keypress→paste latency was not separately measured either: exercising it
needs a live microphone + a loaded model, which we don't have here. The
existing per-step debug logs (`actions.rs`: `TranscribeAction::stop called`
→ `Text pasted successfully in {:?}` → `TranscribeAction::stop completed in
{:?}`) are unaffected by this change, since the destroy/recreate path only
touches the main settings window, never the recording/transcription
pipeline or the recording overlay.

## What we changed

### 1. Destroy the main window's WebView2 on hide-to-tray (Windows only)

`src-tauri/src/lib.rs`:
- Factored the main window's construction (title, size, effects, portable
  data dir, WebView2 accelerator-key tweak, theme) out of `setup()` into
  `build_main_window()`, used both at startup and to recreate the window.
- Added `destroy_main_window_for_memory()` (Windows-only): calls
  `WebviewWindow::destroy()`, which tears down the window's WebView2
  controller/core and its renderer process.
- Wired it into the existing `CloseRequested` handler (which already just
  called `window.hide()` to park the app in the tray): on Windows, after
  hiding, if the closed window is `"main"`, destroy it too.
- `show_main_window()` now recreates the window with `build_main_window()`
  when `get_webview_window("main")` returns `None` (i.e. it was destroyed).

**Why destroy+recreate over `MemoryUsageTargetLevel::Low`/`TrySuspend`:**
those WebView2 APIs (`ICoreWebView2Controller4`/experimental
`ICoreWebView2_X` process-management interfaces) reduce a *suspended*
webview's memory but keep its renderer process (and the associated
GPU/compositor state) alive and are less predictable across WebView2
Evergreen updates; the app already had a battle-tested pattern for creating
this exact window (used at startup) that fully reclaims memory
deterministically. The trade-off is a `build_main_window()` call
(WebView2 controller creation + navigation) the next time the user opens
the window, instead of an instant unhide — WebView2 controller creation is
typically on the order of 150–400 ms depending on `EBWebView` cache warmth;
this wasn't independently timed in this pass (see caveat below), but no
functional issue was observed manually opening/closing the window.

**Windows-only, by design:** macOS's WKWebView and Linux's WebKitGTK
process models don't carry the same ~100+ MB always-on renderer/GPU-process
tax per hidden webview that Chromium/WebView2 does, and destroying/rebuilding
the window risks platform-specific regressions (NSPanel plumbing, GTK
layer-shell/X11 main-thread constraints already called out in `overlay.rs`)
for a cost that isn't the dominant one on those platforms.

**What we found while measuring, and didn't chase:** the app also creates
two more WebView2 windows at startup — the recording overlay
(`utils::create_recording_overlay`) and the live-subtitles overlay
(`live_translate::create_live_subtitles_window`) — both hidden, both kept
alive indefinitely (per the existing comments in `overlay.rs` about issue
#1279 and the need for the overlay to react to `mic-level`/`recording-ready`
events instantly). Their renderer processes, plus one always-shared
GPU/network/storage utility process each webview implicitly needs, account
for most of the ~600 MB that *doesn't* go away when `main` is destroyed.
Making those two windows lazy (create on first recording, destroy after N
seconds idle) would save more memory than this change, but it trades away
the near-zero recording-overlay show latency the current always-resident
design was deliberately built for (see `overlay.rs`'s `show_overlay_state`
and `emit_recording_ready` comments) — that's a latency/memory trade-off
outside this task's "keep behaviour" constraint, so we left it as a
documented opportunity rather than implementing it.

**Caveat on the recreate path:** we verified functionally that (a) closing
the main window drops one WebView2 renderer process and reduces total
working set/private bytes as shown in the table, and (b) the code compiles
and `cargo test --lib` passes. We did not get a clean instrumented
measurement of the recreate-on-reopen latency in this environment (spawning
a second process to trigger the single-instance show-window path raced with
this build's own startup instead of reusing the running instance, which is
an environment/build artifact unrelated to this change — the single-instance
plugin and tray click handler both call the same `show_main_window()`).

### 2. Frontend: lazy-load sidebar sections (code splitting)

`src/components/Sidebar.tsx`: `SECTIONS_CONFIG[*].component` for every
section except `debug` (home, general, history, scratchpad, models,
advanced, snippets, transforms, postprocessing, style, liveTranslate,
copilot, about) now resolves through `React.lazy(() => import(...))`
instead of a static barrel import. `debug` stays eager because
`App.tsx` already imports `DebugSettings` directly for the onboarding-preview
flow, so it's unconditionally on the initial bundle regardless.

`src/App.tsx`: wrapped the section-content render site in `<Suspense
fallback={null}>` so switching sections the first time doesn't throw.

Measured effect (`bun run build`, production Vite build): the following
section chunks are now separate, lazily-fetched JS files instead of being
part of the initial bundle:

| Chunk | Size (min) | gzip |
| --- | --- | --- |
| HomeSettings | 21.6 kB | 5.6 kB |
| ModelsSettings | 11.1 kB | 3.6 kB |
| HistorySettings | 9.3 kB | 3.6 kB |
| AdvancedSettings | 7.6 kB | 2.2 kB |
| GeneralSettings | 5.3 kB | 2.0 kB |
| AboutSettings | 4.3 kB | 1.4 kB |
| CopilotSettings | 3.2 kB | 1.3 kB |
| LiveTranslateSettings | 1.7 kB | 0.8 kB |
| StyleSettings | 1.4 kB | 0.7 kB |
| SnippetsSettings, TransformsSettings | <0.5 kB each | — |

(`PostProcessingSettings` is also dynamically imported from `Sidebar.tsx`,
but Vite/Rollup keeps it merged into the main chunk because two other
files — `PostProcessingSettingsApi/index.tsx` and
`PostProcessingSettingsPrompts.tsx` — still import it statically; splitting
that further was out of scope.)

Only the active section's code now needs to be parsed/compiled by
WebView2's JS engine on first visit, instead of all thirteen sections
up front. We did not capture a "before" main-chunk size for a byte-exact
delta (would have required a throwaway revert+rebuild); the chunk table
above is the concrete, reproducible evidence of what moved out of the
initial bundle.

## What we reviewed and did not change

- **WebView2 browser args for overlay windows** (e.g. disabling GPU
  compositing): the recording overlay and live-subtitles overlay share a
  single GPU process with the main window (confirmed via `Win32_Process`
  command-line inspection — one `--type=gpu-process` helper serves all three
  windows), so per-window GPU-disable flags wouldn't remove that process,
  only degrade the overlay's own animation smoothness. Not applied.
- **Overlay windows created lazily / destroyed when hidden**: see "What we
  found" above — real savings, but a latency trade-off the brief's "keep
  behaviour" constraint argues against making unilaterally. Left as a
  documented follow-up.
- **`memory::trim_freed_memory` / model unload defaults / idle threads**:
  already implemented on the Rust side (glibc-only malloc trim after each
  transcription, 5-minute default model unload timeout with a 10s-interval
  idle watcher thread). No changes needed; verified by reading
  `src-tauri/src/memory.rs` and `src-tauri/src/managers/transcription.rs`.
- **Frontend dependencies**: `package.json` has no unused/heavy packages to
  drop (all listed frontend deps are exercised by existing sections we
  reviewed while wiring the lazy imports).

## Reproducing

```powershell
bun install
bun run build
$env:CMAKE_BUILD_PARALLEL_LEVEL = "4"  # workaround for an MSBuild child-node crash on some machines
bun run tauri build --no-bundle
# Launch target\release\handy.exe (or the target-triple dir tauri prints), then:
.\scripts\measure-ram.ps1 -Label "window-open"
# ... hide the window to the tray, then:
.\scripts\measure-ram.ps1 -Label "hidden-to-tray"
```
