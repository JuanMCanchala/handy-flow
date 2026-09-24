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
| Window open | 7 | 532.3 MB | 347.0 MB |
| **Hidden to tray (main window's WebView2 destroyed)** | 6 | **415.1 MB** | **269.9 MB** |

(Process count dropped from 9 in the original brief's baseline to 7 mainly
because the live-subtitles overlay is no longer created at startup — see
"2. Make the live-subtitles overlay lazy" below; it used to add a permanent
WebView2 renderer. The 7→6 drop on hide is the main-window destroy from the
first pass.)

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

**What we found while measuring, and didn't chase further:** the app also
creates the recording overlay (`utils::create_recording_overlay`) at
startup, hidden, kept alive indefinitely (per the existing comments in
`overlay.rs` about issue #1279 and the need to react to
`mic-level`/`recording-ready` events instantly). Its renderer process, plus
the always-shared GPU/network/storage utility processes every webview
implicitly needs, account for most of the memory that doesn't go away when
`main` is destroyed. We *did* act on the second always-resident window we
found (the live-subtitles overlay) — see "2. Make the live-subtitles overlay
lazy" below — but left the recording overlay resident, since instant-show
latency is core to its purpose (see "3. Recording overlay tweaks" for what
we checked there).

**Caveat on the recreate path:** we verified functionally that (a) closing
the main window drops one WebView2 renderer process and reduces total
working set/private bytes as shown in the table (confirmed via
`Win32_Process` command-line inspection of the surviving helpers' `--type=`
argument, and via the app's own debug log), and (b) the code compiles and
`cargo test --lib` passes. We did not get a clean instrumented measurement
of the recreate-on-reopen latency in this environment (spawning a second
process to trigger the single-instance show-window path raced with this
build's own startup instead of reusing the running instance, which is an
environment/build artifact unrelated to this change — the single-instance
plugin and tray click handler both call the same `show_main_window()`).

### 2. Make the live-subtitles overlay lazy (create on session start, destroy on stop)

`src-tauri/src/live_translate/overlay.rs`: `create_live_subtitles_window`
now shows the window if it already exists, otherwise builds it (previously
it only ever built it, hidden, and was called once at startup). Added
`destroy_live_subtitles_window`, which emits `live-subtitles-hide` (so any
in-flight content is cleared before the window disappears — moot once
destroyed, kept as a no-op-safe courtesy for the brief moment the event
travels) and calls `WebviewWindow::destroy()`.

`src-tauri/src/live_translate/pipeline.rs`: `LiveTranslateManager`'s
`start_with_mode` (used by both live subtitles and copilot — they share this
one capture/VAD/transcription pipeline, see the module doc comment) now
calls `create_live_subtitles_window` instead of `show`; `stop` calls
`destroy_live_subtitles_window` instead of `hide`.

`src-tauri/src/lib.rs`: removed the startup call
(`live_translate::create_live_subtitles_window(app_handle)`) from
`initialize_core_logic`. Only the recording overlay is still created eagerly
at boot.

This applies on Windows, macOS, and Linux alike — unlike the main-window
change, `WebviewWindow::destroy()` here isn't gated to Windows, since the
live-subtitles window has no instant-show requirement on any platform (a
session start already goes through opening a capture stream, building a VAD
detector, and clearing history state before the window is even touched — a
WebView2/WebKit webview creation is not the bottleneck). Measured effect:
process count at startup dropped from 8 to 7 (confirmed via the app's debug
log — `Recording overlay window created successfully (hidden)` now appears
alone, with no matching "Live subtitles window created" line), and the
window-open total dropped from 648.1 MB to 532.3 MB working set (~116 MB) on
this machine, since that renderer (plus its share of the runtime that would
otherwise have had to serve a 3rd, always-idle webview) is never created
until a live-subtitles/copilot session actually starts.

We did not add a timed idle-destroy for an *inactive* session — the window
already only exists while `LiveTranslateManager.active` is true, and `stop`
is called deterministically (shortcut release, mode switch, or app-level
stop), so there's no "idle but still open" state to time out.

### 3. Recording overlay: reviewed window-property tweaks, none were measurable

Checked all three properties named in the follow-up against the current
`src-tauri/src/overlay.rs`:

- **Smaller initial size**: already as small as the UI needs —
  `OVERLAY_WIDTH`/`OVERLAY_HEIGHT` are 256×50 logical px (grows to 400×120
  only for the streaming state). A WebView2 renderer process's baseline
  memory (Chromium's V8 isolate, Blink/CC compositor bookkeeping, IPC
  buffers) is dominated by the runtime itself, not by the pixel dimensions
  of what it's told to render. We didn't have a way to isolate this
  variable in this pass — comparing the idle (256×50) overlay against the
  streaming (400×120) size needs a live recording/streaming session with a
  loaded model, which this checkout doesn't have (see the model-loaded
  caveat above) — so this is a reasoned expectation from how Chromium's
  renderer process memory is dominated by runtime overhead rather than
  paint-surface size, not a directly measured result.
- **`transparent` + no shadow**: already applied (`.shadow(false)` and
  `.transparent(true)` were already on the builder before this task).
  Nothing to change.
- **Disabling DevTools**: not applicable — this build doesn't enable
  Tauri's `devtools` Cargo feature (checked `src-tauri/Cargo.toml`'s
  `tauri` dependency feature list), so DevTools/inspector support is
  already compiled out of the binary entirely in this release build; there
  is no `.devtools(...)` builder call to add and no runtime toggle to
  measure.

No code change made here — every lever either was already applied or had
nothing to measure against.

### 4. Frontend: lazy-load sidebar sections (code splitting)

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
  compositing): the recording overlay and (while active) the live-subtitles
  overlay share a single GPU process with the main window (confirmed via
  `Win32_Process` command-line inspection — one `--type=gpu-process` helper
  serves every webview), so per-window GPU-disable flags wouldn't remove
  that process, only degrade the overlay's own animation smoothness. Not
  applied.
- **Recording overlay made lazy like the live-subtitles overlay**: not
  applied, deliberately — instant show/hide on every recording start/stop
  is core to its purpose (see `overlay.rs`'s `show_overlay_state` and
  `emit_recording_ready` comments), and recreating a WebView2 window costs
  on the order of 150–400 ms. That's a latency regression on the app's
  hottest path, so it stays resident. See section 3 above for the narrower
  property-level tweaks we did check for it.
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
