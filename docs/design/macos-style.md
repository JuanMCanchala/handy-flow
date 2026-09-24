# Handy — macOS-style visual system

Status: proposal · Scope: main window (`src/`) + recording overlay (`src/overlay/`)
References: Wispr Flow desktop, Granola, Superwhisper, Raycast, Linear, Things 3, Arc, Craft, Notion Calendar.

> Repo facts this spec is written against: React **18.3** (not 19), Tailwind **v4.1** via `@tailwindcss/vite`,
> tokens in `src/styles/theme.css` registered in `src/App.css` with `@theme inline`, theming via
> `data-theme` on `<html>` (`src/lib/utils/theme.ts`), `data-platform` set in `src/main.tsx`,
> main window **built in Rust** (`src-tauri/src/lib.rs`, `tauri.conf.json` has `"windows": []`),
> `macOSPrivateApi: true` already enabled, 24 locales incl. RTL (ar, he) and CJK/Devanagari.

---

## 1. Principles

1. **Quiet chrome, loud content.** The window is a warm, slightly tinted canvas; content sits on one big white panel. No colored surfaces except state (error / warning / recording).
2. **Ink, not candy.** Primary actions are near-black ink (light) / near-white (dark). The pink brand color survives only in the logo and the recording indicator.
3. **One elevation step.** Canvas → panel → (popover/dialog). Separation comes from hairlines (1px, ~8% ink) and a soft ambient shadow, never from heavy borders.
4. **Editorial contrast.** A serif display face only for big numbers and page titles; everything else is a neutral sans at 13–14px.
5. **Generous, regular spacing.** 4px grid, 24–32px panel padding, 44–52px row heights.
6. **Native feel.** Hidden titlebar with drag region, traffic-light inset, vibrancy/Mica behind the sidebar, `cursor: default`, no text selection on chrome, fast (120–200ms) motion that honours reduced-motion.
7. **Behaviour and i18n untouched.** This is a skin: no copy changes, no new keys except where a new visible string is introduced (then add to every `src/i18n/locales/*/translation.json`). Use logical properties (`ps-`, `pe-`, `start-`, `end-`, `border-e`) so RTL keeps working.

## 2. Typography

| Role          | Family (stack)                                                                                                                                                                                                                                                                         | Notes                                                                                                                                                                                                                                                                                                                                   |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| UI sans       | `"Inter Variable", -apple-system, BlinkMacSystemFont, "Segoe UI Variable Text", "Segoe UI", system-ui, "Noto Sans", "PingFang SC", "Hiragino Sans", "Microsoft YaHei UI", "Yu Gothic UI", "Malgun Gothic", "Noto Sans Arabic", "Noto Sans Hebrew", "Noto Sans Devanagari", sans-serif` | Inter (OFL) bundled via `@fontsource-variable/inter` (MIT package, offline, no CDN). On macOS use the system font first (`:root[data-platform="macos"]` swaps the stack to `-apple-system` → SF Pro) — it is what Things/Craft/Raycast use. Enable `font-feature-settings: "cv11", "ss01", "tnum" 0` for Inter; `tnum` only on numbers. |
| Display serif | `"Instrument Serif", "Newsreader", "Iowan Old Style", "Palatino", Georgia, serif`                                                                                                                                                                                                      | Instrument Serif (OFL, 400 + italic only) via `@fontsource/instrument-serif`. **Only** for stat numbers and the page H1, never below 28px. Wispr Flow does the same pairing (serif ≥32px, sans ≤24px).                                                                                                                                  |
| Mono          | `ui-monospace, "SF Mono", "Cascadia Code", "JetBrains Mono", Menlo, Consolas, monospace`                                                                                                                                                                                               | Shortcuts, paths, log viewer.                                                                                                                                                                                                                                                                                                           |

Non-Latin locales: Instrument Serif has Latin only. For `:lang(ar, he, hi, ne, ja, ko, zh, ru, uk, bg)` set `--font-display` to the sans stack at weight 600 (Cyrillic could use Newsreader, but keep it simple and consistent).

### Type scale (root stays 15px in App.css today → change to 14px; all values in px for clarity)

| Token        | Size / line-height | Weight    | Tracking          | Use                                                                    |
| ------------ | ------------------ | --------- | ----------------- | ---------------------------------------------------------------------- |
| `display-xl` | 44 / 48            | 400 serif | -0.01em           | Stat number ("6,808")                                                  |
| `display`    | 32 / 38            | 400 serif | -0.01em           | Page H1 ("History", "General")                                         |
| `title`      | 17 / 24            | 600       | -0.01em           | Card / dialog title                                                    |
| `body`       | 14 / 20            | 400       | -0.003em          | Default text, row labels (500)                                         |
| `small`      | 13 / 18            | 400       | 0                 | Descriptions, secondary                                                |
| `caption`    | 12 / 16            | 500       | 0                 | Meta, timestamps, footer                                               |
| `overline`   | 11 / 14            | 600       | 0.06em, uppercase | Section headers (only Latin; keep sentence-case otherwise is fine too) |

Secondary text = `--color-text-secondary`, tertiary = `--color-text-tertiary`; never use opacity on text for hierarchy (it breaks on vibrancy).

## 3. Tokens

Replace the contents of `src/styles/theme.css` with the block below (keeps the file's "light/dark pair defined once" pattern and the `data-theme` override contract that `theme.ts` and the overlay rely on). Legacy tokens are **aliased** so the ~42 files using `bg-background`, `text-text`, `border-mid-gray/20`, `bg-logo-primary`, `bg-background-ui` keep compiling and immediately look closer to the target; they are swept in Phase 3.

```css
/* src/styles/theme.css */
:root {
  /* ---------- Light ---------- */
  --light-canvas: #f7f6f3; /* window / sidebar background (warm off-white) */
  --light-canvas-vibrant: rgb(
    247 246 243 / 0.72
  ); /* when a native effect is active */
  --light-surface: #ffffff; /* content panel, cards */
  --light-surface-raised: #ffffff; /* popovers, dialogs */
  --light-surface-sunken: #f2f1ed; /* inputs, code/log, segmented track */
  --light-fill-hover: rgb(28 27 23 / 0.045);
  --light-fill-active: rgb(28 27 23 / 0.075); /* sidebar active pill, pressed */
  --light-fill-selected: rgb(28 27 23 / 0.06);
  --light-text: #1c1b17;
  --light-text-secondary: #6b6962;
  --light-text-tertiary: #9c9a93;
  --light-text-on-accent: #ffffff;
  --light-border: rgb(28 27 23 / 0.08); /* hairlines, dividers */
  --light-border-strong: rgb(28 27 23 / 0.14); /* inputs, buttons */
  --light-accent: #1c1b17; /* primary button, toggle on, focus ring base */
  --light-accent-hover: #34332e;
  --light-focus: rgb(28 27 23 / 0.28);
  --light-brand: #e2669d; /* logo + recording dot only */
  --light-success: #2f7d4f;
  --light-warning: #b4690e;
  --light-error: #c93a2f;
  --light-shadow-panel:
    0 0 0 1px rgb(28 27 23 / 0.06), 0 1px 2px rgb(28 27 23 / 0.04),
    0 8px 24px -8px rgb(28 27 23 / 0.08);
  --light-shadow-pop:
    0 0 0 1px rgb(28 27 23 / 0.08), 0 4px 12px rgb(28 27 23 / 0.08),
    0 16px 40px -12px rgb(28 27 23 / 0.18);

  /* ---------- Dark ---------- */
  --dark-canvas: #1b1a18;
  --dark-canvas-vibrant: rgb(27 26 24 / 0.6);
  --dark-surface: #242321;
  --dark-surface-raised: #2c2b28;
  --dark-surface-sunken: #1f1e1c;
  --dark-fill-hover: rgb(255 253 245 / 0.05);
  --dark-fill-active: rgb(255 253 245 / 0.09);
  --dark-fill-selected: rgb(255 253 245 / 0.07);
  --dark-text: #ecebe6;
  --dark-text-secondary: #a09e97;
  --dark-text-tertiary: #6f6d67;
  --dark-text-on-accent: #1b1a18;
  --dark-border: rgb(255 253 245 / 0.08);
  --dark-border-strong: rgb(255 253 245 / 0.14);
  --dark-accent: #ecebe6;
  --dark-accent-hover: #ffffff;
  --dark-focus: rgb(236 235 230 / 0.35);
  --dark-brand: #f28cbb;
  --dark-success: #5fbf85;
  --dark-warning: #e6a84a;
  --dark-error: #f07f74;
  --dark-shadow-panel:
    0 0 0 1px rgb(255 253 245 / 0.06), 0 1px 2px rgb(0 0 0 / 0.3),
    0 12px 32px -12px rgb(0 0 0 / 0.5);
  --dark-shadow-pop:
    0 0 0 1px rgb(255 253 245 / 0.08), 0 8px 24px rgb(0 0 0 / 0.35),
    0 24px 48px -16px rgb(0 0 0 / 0.55);

  /* ---------- Theme-independent ---------- */
  --radius-xs: 4px; /* kbd, badges-inside-rows */
  --radius-sm: 6px; /* inputs, small buttons, menu items */
  --radius-md: 8px; /* buttons, sidebar pill, dropdown menu */
  --radius-lg: 12px; /* cards, dialogs */
  --radius-xl: 16px; /* content panel */
  --radius-full: 9999px;

  --ease-out: cubic-bezier(0.22, 1, 0.36, 1);
  --ease-in-out: cubic-bezier(0.65, 0, 0.35, 1);
  --ease-spring: cubic-bezier(0.34, 1.4, 0.64, 1); /* toggles, pill only */
  --dur-fast: 120ms; /* hover, color */
  --dur-base: 180ms; /* popover/menu, toggle */
  --dur-slow: 260ms; /* dialog, overlay pill resize */

  --sidebar-w: 216px;
  --titlebar-h: 44px; /* drag strip height */
  --traffic-light-inset: 76px; /* macOS: space reserved left of the window controls */

  /* ---------- Active palette (default light) ---------- */
  --color-canvas: var(--light-canvas);
  --color-canvas-vibrant: var(--light-canvas-vibrant);
  --color-surface: var(--light-surface);
  --color-surface-raised: var(--light-surface-raised);
  --color-surface-sunken: var(--light-surface-sunken);
  --color-fill-hover: var(--light-fill-hover);
  --color-fill-active: var(--light-fill-active);
  --color-fill-selected: var(--light-fill-selected);
  --color-text: var(--light-text);
  --color-text-secondary: var(--light-text-secondary);
  --color-text-tertiary: var(--light-text-tertiary);
  --color-text-on-accent: var(--light-text-on-accent);
  --color-border: var(--light-border);
  --color-border-strong: var(--light-border-strong);
  --color-accent: var(--light-accent);
  --color-accent-hover: var(--light-accent-hover);
  --color-focus: var(--light-focus);
  --color-brand: var(--light-brand);
  --color-success: var(--light-success);
  --color-warning: var(--light-warning);
  --color-error: var(--light-error);
  --shadow-panel: var(--light-shadow-panel);
  --shadow-pop: var(--light-shadow-pop);

  /* ---------- Legacy aliases (remove in Phase 3) ---------- */
  --color-background: var(--color-surface);
  --color-background-ui: var(
    --color-accent
  ); /* was pink #da5893: primary buttons, toggle-on */
  --color-logo-primary: var(
    --color-brand
  ); /* logo fill; hover/active usages swept in Phase 3 */
  --color-logo-stroke: var(--color-text);
  --color-text-stroke: var(--color-surface);
  --color-mid-gray: #8c8a83; /* warm gray so /10 /20 opacities still read as hairlines */
}

/* Dark: the same list of assignments, pointed at --dark-*.
   Emit it three times exactly like today's theme.css:
   1) @media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { … } }
   2) :root[data-theme="dark"] { … }
   3) :root[data-theme="light"] { … --light-* … }   (explicit light wins over a dark OS)
   Tip: put the assignments in a single comment-marked block and copy; do not introduce a CSS preprocessor. */
```

Register utilities in `src/App.css` (replace the current `@theme inline` block):

```css
@import "tailwindcss";
@import "@fontsource-variable/inter";
@import "@fontsource/instrument-serif/400.css";
@import "@fontsource/instrument-serif/400-italic.css";
@import "./styles/theme.css";

@custom-variant dark (&:where([data-theme="dark"], [data-theme="dark"] *));

@theme inline {
  /* colors */
  --color-canvas: var(--color-canvas);
  --color-surface: var(--color-surface);
  --color-surface-raised: var(--color-surface-raised);
  --color-surface-sunken: var(--color-surface-sunken);
  --color-fill-hover: var(--color-fill-hover);
  --color-fill-active: var(--color-fill-active);
  --color-fill-selected: var(--color-fill-selected);
  --color-text: var(--color-text);
  --color-text-secondary: var(--color-text-secondary);
  --color-text-tertiary: var(--color-text-tertiary);
  --color-on-accent: var(--color-text-on-accent);
  --color-border: var(--color-border);
  --color-border-strong: var(--color-border-strong);
  --color-accent: var(--color-accent);
  --color-accent-hover: var(--color-accent-hover);
  --color-focus: var(--color-focus);
  --color-brand: var(--color-brand);
  --color-success: var(--color-success);
  --color-warning: var(--color-warning);
  --color-error: var(--color-error);
  /* legacy (Phase 3 removes) */
  --color-background: var(--color-background);
  --color-background-ui: var(--color-background-ui);
  --color-logo-primary: var(--color-logo-primary);
  --color-logo-stroke: var(--color-logo-stroke);
  --color-text-stroke: var(--color-text-stroke);
  --color-mid-gray: var(--color-mid-gray);

  /* type */
  --font-sans:
    "Inter Variable", -apple-system, BlinkMacSystemFont,
    "Segoe UI Variable Text", "Segoe UI", system-ui, "Noto Sans", "PingFang SC",
    "Hiragino Sans", "Microsoft YaHei UI", "Yu Gothic UI", "Malgun Gothic",
    "Noto Sans Arabic", "Noto Sans Hebrew", "Noto Sans Devanagari", sans-serif;
  --font-display:
    "Instrument Serif", "Newsreader", "Iowan Old Style", Palatino, Georgia,
    serif;
  --font-mono:
    ui-monospace, "SF Mono", "Cascadia Code", "JetBrains Mono", Menlo, Consolas,
    monospace;
  --text-display-xl: 44px;
  --text-display-xl--line-height: 48px;
  --text-display-xl--letter-spacing: -0.01em;
  --text-display: 32px;
  --text-display--line-height: 38px;
  --text-display--letter-spacing: -0.01em;
  --text-title: 17px;
  --text-title--line-height: 24px;
  --text-body: 14px;
  --text-body--line-height: 20px;
  --text-small: 13px;
  --text-small--line-height: 18px;
  --text-caption: 12px;
  --text-caption--line-height: 16px;
  --text-overline: 11px;
  --text-overline--line-height: 14px;

  /* shape / depth / motion */
  --radius-xs: var(--radius-xs);
  --radius-sm: var(--radius-sm);
  --radius-md: var(--radius-md);
  --radius-lg: var(--radius-lg);
  --radius-xl: var(--radius-xl);
  --shadow-panel: var(--shadow-panel);
  --shadow-pop: var(--shadow-pop);
  --ease-out: var(--ease-out);
  --ease-in-out: var(--ease-in-out);
}
```

Note: the repo's dark mode is attribute/media driven, not the `.dark` class. Tokens flip on their own, so components should use semantic utilities (`bg-surface`, `text-text-secondary`) and almost never `dark:`. The `@custom-variant` line is only for the rare explicit case; if used it must also cover the `system` + OS-dark case, so prefer tokens.

Base layer additions in `App.css`:

```css
:root {
  font-size: 14px;
  line-height: 20px;
  font-family: var(--font-sans);
  font-feature-settings: "cv11", "ss01";
  background: var(--color-canvas);
}
:root[data-platform="macos"] {
  --font-sans:
    -apple-system, BlinkMacSystemFont, "Inter Variable", system-ui, sans-serif;
}
:root:lang(ar),
:root:lang(he),
:root:lang(hi),
:root:lang(ne),
:root:lang(ja),
:root:lang(ko),
:root:lang(zh),
:root:lang(ru),
:root:lang(uk),
:root:lang(bg) {
  --font-display: var(--font-sans);
}
.tabular {
  font-variant-numeric: tabular-nums;
}
:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
  border-radius: inherit;
}
::selection {
  background: color-mix(in srgb, var(--color-brand) 28%, transparent);
}
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 1ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 1ms !important;
    scroll-behavior: auto !important;
  }
}
[data-tauri-drag-region] {
  -webkit-app-region: drag;
  app-region: drag;
}
[data-tauri-drag-region]
  :is(button, a, input, select, textarea, [role="button"]) {
  -webkit-app-region: no-drag;
}
```

Scrollbar: keep the existing custom scrollbar, but thumb = `var(--color-border-strong)`, hover = `var(--color-text-tertiary)`, width 10px.

## 4. Window chrome (Tauri 2)

The main window is created in `src-tauri/src/lib.rs` (`WebviewWindowBuilder::new(app, "main", …)`), so change it there; `tauri.conf.json` stays `"windows": []`. Equivalent JSON (for reference / if it is ever moved to config):

```jsonc
{
  "label": "main",
  "title": "Handy",
  "width": 880,
  "height": 620,
  "minWidth": 720,
  "minHeight": 540,
  "titleBarStyle": "Overlay", // macOS: content under the titlebar, traffic lights kept
  "hiddenTitle": true, // macOS
  "trafficLightPosition": { "x": 18, "y": 22 }, // macOS; requires Overlay + decorations
  "transparent": true, // needed by windowEffects (macOS uses macOSPrivateApi, already on)
  "windowEffects": {
    "effects": ["sidebar", "mica"],
    "state": "followsWindowActiveState",
  },
}
```

Rust (sketch — guard per platform, keep portable `data_directory` and the WebView2 accelerator code as is):

```rust
use tauri::utils::config::WindowEffectsConfig;
use tauri::window::{Effect, EffectState};

let mut win_builder = tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("/".into()))
    .title("Handy")
    .inner_size(880.0, 620.0)
    .min_inner_size(720.0, 540.0)
    .resizable(true).maximizable(true).visible(false);

#[cfg(target_os = "macos")]
{
    win_builder = win_builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(tauri::LogicalPosition::new(18.0, 22.0)) // verify the builder method exists in tauri 2.11 (config key `trafficLightPosition` does); else set it via `window.set_traffic_light_position` / objc after build
        .transparent(true)
        .effects(WindowEffectsConfig { effects: vec![Effect::Sidebar],
                 state: Some(EffectState::FollowsWindowActiveState), radius: None, color: None });
}
#[cfg(target_os = "windows")]
{
    // Win11 22H2+: Mica. Older Windows ignores the effect -> CSS falls back to the opaque canvas.
    win_builder = win_builder
        .transparent(true)
        .effects(WindowEffectsConfig { effects: vec![Effect::Mica], state: None, radius: None, color: None });
}
```

Rules that go with it:

- **Only the sidebar/canvas is translucent.** `body` background becomes `transparent` when an effect is active; the sidebar uses `--color-canvas-vibrant`; the content panel is always opaque `--color-surface`. Expose the state to CSS: set `document.documentElement.dataset.vibrancy = "on"` from `main.tsx` when platform is `macos`, or `windows` and `os.version()` build ≥ 22000 (`@tauri-apps/plugin-os` is already a dependency). CSS: `:root[data-vibrancy="on"] body { background: transparent }`, otherwise `background: var(--color-canvas)`. Linux: no effects, opaque canvas.
- **Theme sync**: `apply_window_theme` (`src-tauri/src/shortcut/mod.rs`) already sets the native theme; Mica/vibrancy follow it (use `Effect::Mica` not `MicaLight/MicaDark` so it tracks the theme).
- **Drag region**: a `data-tauri-drag-region` strip of `--titlebar-h` across the top of the sidebar and the content panel header. Tauri 2 needs `core:window:allow-start-dragging` in `src-tauri/capabilities/default.json` (not in `core:default`). Double-click on the strip maximizes on macOS natively; on Windows add `core:window:allow-toggle-maximize` and handle `onDoubleClick` → `getCurrentWindow().toggleMaximize()`.
- **macOS traffic lights**: sidebar top padding `--titlebar-h`, logo row starts below them; on macOS nothing may render in the first `--traffic-light-inset` px of the top-left 44px. In RTL the traffic lights stay on the left (physical), so use `left`, not `start`, for this one inset.
- **Windows**: keep native decorations (caption buttons + snap layouts) — do not draw fake controls. With Mica the native caption area picks up the effect. If the white-flash/resize lag of `transparent(true)` shows up on Windows 10, drop `transparent` + effects there (feature-detect as above).
- Overlay window (`src-tauri/src/overlay.rs`) is already transparent/undecorated; no chrome change.

## 5. Layout

```
┌───────────────── window (canvas #f7f6f3 / vibrancy) ─────────────────┐
│ ● ● ●   (44px drag strip)                                             │
│ ┌ sidebar 216px ┐ ┌──────── content panel (white, r=16) ───────────┐ │
│ │ Handy logo     │ │  H1 serif 32px              [actions]           │ │
│ │                │ │  ─────────────────────────────── hairline      │ │
│ │ ◻ General      │ │  stat cards / settings groups / history list   │ │
│ │ ◼ History ◀pill│ │                                                 │ │
│ │ ◻ Models       │ │  max-width 720px, centered, padding 32px        │ │
│ │ …              │ │                                                 │ │
│ │ ── footer ──── │ │                                                 │ │
│ │ model · v0.9.7 │ └─────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────────┘
```

- Outer gap: panel inset `8px` top/right/bottom (`m-2 ms-0`), so the canvas frames it — the Wispr Flow / Arc look.
- Panel: `bg-surface rounded-xl shadow-panel overflow-hidden`, inner scroll container `overflow-y-auto`, content `mx-auto w-full max-w-[720px] px-8 pt-6 pb-12 flex flex-col gap-8`.
- Footer (`src/components/footer/Footer.tsx`) moves from the full-width bottom bar into the bottom of the sidebar (model selector as a compact ghost button, version + update in `text-caption text-text-tertiary`). Behaviour unchanged.

## 6. Component specs

### 6.1 Sidebar (`src/components/Sidebar.tsx`)

- Container: `w-[var(--sidebar-w)] shrink-0 h-full flex flex-col bg-transparent` (canvas shows through), `pt-[var(--titlebar-h)] px-3 pb-3`, no right border (the panel's shadow separates).
- Logo: `HandyTextLogo` width 88, `ms-2 mb-6`, inside the drag region.
- Item: `<button>` (not `div` — gives keyboard + focus for free) `h-8 w-full flex items-center gap-2.5 px-2.5 rounded-md text-body font-medium text-text-secondary transition-colors duration-[var(--dur-fast)]`
  - hover: `bg-fill-hover text-text`
  - active: `bg-fill-active text-text` + `aria-current="page"`; optional 1px inner highlight in light mode: `shadow-[inset_0_0_0_1px_var(--color-border)]`
  - icon: lucide at `size={16} strokeWidth={1.75}`, `text-text-tertiary` → `text-text` when active. `HandyHand` for General stays but rendered at 16px monochrome (`currentColor`), or swap to lucide `Home`.
- Spacing: `gap-0.5` between items; optional group label (`text-overline text-text-tertiary px-2.5 mt-4 mb-1`) only if sections are grouped later.
- Bottom: footer block with `border-t border-border pt-3 mt-auto`.

### 6.2 Page header (new `src/components/ui/PageHeader.tsx`)

- `h1` `font-display text-display text-text` + optional `p` `text-small text-text-secondary mt-1`, right-aligned actions slot. Title text = existing `t(section.labelKey)`; no new keys.
- Wrapper carries `data-tauri-drag-region` (actions are auto no-drag via CSS rule).

### 6.3 Stat card (new `src/components/ui/StatCard.tsx`)

- `rounded-lg bg-surface border border-border px-5 py-4 flex flex-col gap-1` (on the panel it is border-only, no shadow).
- Number: `font-display text-display-xl tabular text-text` formatted with `Intl.NumberFormat(i18n.language)` → "6,808" / "6.808".
- Label: `text-small text-text-secondary` ("total words"). New label strings need keys in all locales; data source (history rows word counts / count) is a separate feature — the component is presentational only.
- Row of 2–4 cards: `grid grid-cols-[repeat(auto-fit,minmax(160px,1fr))] gap-3`.

### 6.4 Settings group & row (`ui/SettingsGroup.tsx`, `ui/SettingContainer.tsx`)

- Group title: `text-overline uppercase text-text-tertiary px-1 mb-2` (was `text-xs text-mid-gray uppercase`). For non-Latin locales uppercase is a no-op, fine.
- Group body: `rounded-lg bg-surface border border-border divide-y divide-border` (no shadow inside the panel).
- Row (horizontal layout): `min-h-[52px] px-4 py-3 flex items-center justify-between gap-6`; title `text-body font-medium`; inline description `text-small text-text-secondary mt-0.5`; tooltip `Info` icon `size-3.5 text-text-tertiary hover:text-text`.
- Disabled: `opacity-50` on the whole row (keep current logic).

### 6.5 History list row (`settings/history/HistorySettings.tsx`)

- List: same group container; rows `group relative px-4 py-3.5 flex flex-col gap-1.5 hover:bg-fill-hover transition-colors`.
- Meta line: timestamp `text-caption text-text-tertiary tabular` (formatted date stays as today), optional word count chip.
- Text: `text-body text-text leading-6 line-clamp-3` (expand on click if already supported; otherwise unchanged).
- Actions (copy, star, retry, delete — existing buttons): `absolute top-2.5 end-3 flex gap-0.5 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity duration-[var(--dur-fast)]`; each is an icon button (6.6 `icon`), delete uses `hover:text-error`. Keep them keyboard reachable (focus-within reveals them), keep `aria-label`s.
- Audio player: `ui/AudioPlayer.tsx` restyled to a slim row: track `h-1 rounded-full bg-border-strong`, progress `bg-text`, play button 24px circle `bg-fill-active`.
- Day grouping (optional): sticky `text-overline text-text-tertiary bg-surface/90 backdrop-blur px-4 py-2` headers.

### 6.6 Buttons (`ui/Button.tsx`) — keep the variant names so call sites don't change

Base: `inline-flex items-center justify-center gap-1.5 font-medium rounded-md border transition-[background-color,border-color,color,box-shadow] duration-[var(--dur-fast)] disabled:opacity-40 disabled:pointer-events-none cursor-default focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus` (drop `focus:outline-none` / `focus:ring-*`).

| Variant        | Classes                                                                                                |
| -------------- | ------------------------------------------------------------------------------------------------------ |
| `primary`      | `bg-accent text-on-accent border-transparent hover:bg-accent-hover shadow-[0_1px_1px_rgb(0_0_0/0.12)]` |
| `primary-soft` | `bg-fill-active text-text border-transparent hover:bg-fill-selected`                                   |
| `secondary`    | `bg-surface text-text border-border-strong hover:bg-fill-hover shadow-[0_1px_1px_rgb(0_0_0/0.04)]`     |
| `ghost`        | `bg-transparent text-text-secondary border-transparent hover:bg-fill-hover hover:text-text`            |
| `warning`      | secondary + `hover:border-warning hover:text-warning`                                                  |
| `danger`       | `bg-error text-white border-transparent hover:brightness-95`                                           |
| `danger-ghost` | ghost + `text-error hover:bg-error/10`                                                                 |

Sizes: `sm` `h-7 px-2.5 text-caption`, `md` `h-8 px-3.5 text-small`, `lg` `h-9 px-4 text-body`. Add `icon` size: `size-7 p-0` (used by history actions, dialog close). `ResetButton.tsx` → ghost icon button.

### 6.7 Inputs (`ui/Input.tsx`, `ui/Textarea.tsx`, shortcut inputs)

- `h-8 px-3 text-body font-normal bg-surface-sunken border border-transparent rounded-sm placeholder:text-text-tertiary transition-colors hover:border-border-strong focus:bg-surface focus:border-border-strong focus:outline-none focus-visible:shadow-[0_0_0_3px_var(--color-focus)]` (drop `font-semibold` and the pink hovers). `compact`: `h-7 px-2 text-small`.
- Textarea: same, `py-2 min-h-[96px] leading-6`, `resize-y`.
- `GlobalShortcutInput.tsx` / `HandyKeysShortcutInput.tsx`: keys shown as `kbd` chips — `font-mono text-caption px-1.5 h-5 rounded-xs bg-surface-sunken border border-border shadow-[inset_0_-1px_0_var(--color-border)]`; recording state: `border-text` + pulsing brand dot (disabled under reduced motion).

### 6.8 Toggle (`ui/ToggleSwitch.tsx`)

- macOS proportions: track `w-[34px] h-5 rounded-full bg-border-strong peer-checked:bg-accent transition-colors duration-[var(--dur-base)]`; knob `after:size-4 after:top-0.5 after:start-0.5 after:bg-white after:rounded-full after:shadow-[0_1px_2px_rgb(0_0_0/0.2)] after:transition-transform after:duration-[var(--dur-base)] after:ease-[var(--ease-spring)] peer-checked:after:translate-x-[14px] rtl:peer-checked:after:-translate-x-[14px]`.
- Dark: `peer-checked:bg-accent` is near-white, so the knob should be `after:bg-[var(--color-surface)]` when checked in dark — simplest: knob color token `--toggle-knob` (`#fff` light, `#1b1a18` dark-checked). Alternative (Linear-style): checked track = `--color-success`-free neutral; don't use pink.
- Focus: `peer-focus-visible:outline-2 peer-focus-visible:outline-focus peer-focus-visible:outline-offset-2` (replace `peer-focus:ring-4 ring-logo-primary`). Spinner: `border-text-tertiary`.

### 6.9 Dropdown / Select (`ui/Dropdown.tsx`, `ui/Select.tsx` react-select `app-select`)

- Trigger: like a `secondary` button: `h-8 px-3 text-body bg-surface border border-border-strong rounded-md hover:bg-fill-hover`, chevron lucide `ChevronsUpDown` 14px `text-text-tertiary` (macOS popup-button cue); no rotation animation.
- Menu: `bg-surface-raised rounded-md shadow-pop p-1 mt-1 max-h-72 overflow-auto`, enter animation `opacity 0→1, translateY(-2px)→0, scale(.98)→1` over `--dur-base` `--ease-out`.
- Item: `h-7 px-2 rounded-sm text-body hover:bg-fill-hover` (with description: `py-1.5 h-auto`), selected: lucide `Check` 14px at end, not a colored background; disabled `opacity-40`.
- Add keyboard support if missing (Up/Down/Enter/Escape) — behaviour-compatible improvement.
- react-select: set `classNames` per part with the same classes (`unstyled: true`), or style `.app-select__*` in `App.css`.

### 6.10 Dialog (`ui/Dialog.tsx`)

- Backdrop: `bg-black/25 dark:bg-black/50 backdrop-blur-[2px]`, fade `--dur-base`.
- Panel: `max-w-[440px] rounded-lg bg-surface-raised shadow-pop border-0`, enter `scale(.97)→1 + opacity` `--dur-slow` `--ease-out`.
- Header: no divider; `px-5 pt-5 pb-2`, title `text-title`, description `text-small text-text-secondary`. Close = `icon` ghost button.
- Footer: no divider; `px-5 pb-5 pt-3 flex justify-end gap-2`, primary action last (macOS order). Keep focus trap, `aria-*`, mask fade as is.

### 6.11 Toasts (sonner config in `src/App.tsx`)

`bg-surface-raised text-text rounded-lg shadow-pop px-4 py-3 text-small gap-3`, description `text-text-secondary`, action button = `secondary sm`. Position `bottom-right` (start in RTL via sonner `dir`).

### 6.12 Tooltip, Badge, Alert, Slider

- Tooltip: `bg-text text-surface text-caption px-2 py-1 rounded-sm shadow-pop`, 300ms open delay, no arrow.
- Badge: `h-5 px-1.5 rounded-xs text-caption font-medium bg-fill-active text-text-secondary`; success `bg-success/12 text-success`.
- Alert / SecureInputWarning: `rounded-lg border border-warning/30 bg-warning/8 px-4 py-3 text-small`; icon in `text-warning`; same for error.
- Slider: track `h-1 rounded-full bg-border-strong`, fill `bg-accent`, thumb `size-4 rounded-full bg-white shadow-[0_0_0_0.5px_rgb(0_0_0/0.2),0_1px_3px_rgb(0_0_0/0.2)]`.

### 6.13 Recording overlay pill (`src/overlay/RecordingOverlay.css`)

Keep all geometry vars (`--ov-*`, synced with `overlay.rs`) and all states. Restyle only:

- `--s-font` → the new `--font-sans` stack.
- Surface: light `rgb(28 27 23 / 0.88)` ink pill with white text (Superwhisper/Wispr Flow style reads on any background); dark `rgb(40 39 36 / 0.9)`. Add `backdrop-filter: blur(20px) saturate(1.4)` (works in the transparent window on macOS/WebView2).
- Border: `0.5px solid rgb(255 255 255 / 0.12)` + `box-shadow: 0 8px 24px rgb(0 0 0 / 0.25)` — make sure the window size in `overlay.rs` leaves room for the shadow, otherwise drop the shadow.
- Radius: `var(--radius-full)`; Live panel expanded: `14px`.
- Accent: waveform bars white at 90%; recording dot `--color-brand`; `--s-accent-soft` → `rgb(255 255 255 / 0.1)`.
- Motion: width/height transitions `--dur-slow var(--ease-out)`; waveform animation disabled under reduced motion (static bars scaled by level).

### 6.14 Onboarding (`src/components/onboarding/`)

Same canvas + a single centered `bg-surface rounded-xl shadow-panel` card, `max-w-[480px] p-8`, serif H1, primary button full width. Keep the `data-onboarding-active` scrollbar-gutter rule.

## 7. Motion & focus summary

- Hover/color: 120ms `ease-out`. Menus/toggles: 180ms. Dialogs/overlay resize: 260ms. No bounces except toggle knob (`--ease-spring`).
- Never animate layout of the list on data refresh; only opacity/transform.
- Focus: `:focus-visible` only, 2px `--color-focus` outline, offset 2px; inputs use a 3px soft ring. Never remove focus without a replacement.
- `prefers-reduced-motion`: global kill-switch (section 3) + overlay waveform static.
- Hit targets ≥ 28px; contrast: `text-secondary` on surface ≥ 4.5:1 (#6b6962 on #fff = 5.4:1; dark #a09e97 on #242321 = 6.1:1); `text-tertiary` only for non-essential meta.

## 8. Open-source references (permissive)

- shadcn/ui themes & `tweakcn` (MIT) — token naming ideas (surface/foreground/border), not adopted wholesale.
- Radix Colors "sand" / "mauve" scales (MIT) — the warm neutrals above sit close to `sand`; can replace hand-picked values.
- Inter (OFL) · Instrument Serif (OFL) · Newsreader (OFL) · @fontsource packages (MIT).
- `window-vibrancy` crate (MIT/Apache, used internally by Tauri effects) — docs on Mica/Acrylic caveats.
- Lucide icons (ISC) — already a dependency; use 16px / 1.75 stroke.
  Avoid copying Wispr Flow's proprietary assets or logos; the look is reproduced from tokens only.

## 9. Migration plan (files → changes)

Each phase is independently shippable; run `bun run build`, `bun run lint`, `bun run check:translations`, `bun run test:playwright` after each.

**Phase 0 — Dependencies**

1. `bun add @fontsource-variable/inter @fontsource/instrument-serif` (fonts bundled, no network at runtime — keeps the app offline-capable and CSP-safe).

**Phase 1 — Tokens (no component edits yet)** 2. `src/styles/theme.css`: replace with section 3 tokens, including the legacy aliases and the three dark/light override blocks. The overlay imports this file too, so verify it still renders. 3. `src/App.css`: font imports, new `@theme inline`, base layer (font, focus, selection, reduced motion, drag region), scrollbar colors; set `--color-log-surface` to `var(--color-surface-sunken)`. Remove the unused `.container` rule only if grep shows no usage. 4. Visual check in light/dark/system on Windows + macOS: at this point pink primary buttons become ink, backgrounds become warm.

**Phase 2 — Shell & chrome** 5. `src-tauri/src/lib.rs`: window size 880×620 (min 720×540), macOS Overlay titlebar + hidden title + traffic lights + `Effect::Sidebar`; Windows Mica (section 4). `src-tauri/capabilities/default.json`: add `core:window:allow-start-dragging` (+ `allow-toggle-maximize` for Windows double-click). 6. `src/main.tsx`: set `data-vibrancy` (platform + Windows build check via `@tauri-apps/plugin-os` `version()`). 7. `src/App.tsx` (main branch of `content` only): root `h-screen flex bg-transparent`; `<Sidebar>` then `<main className="flex-1 m-2 ms-0 bg-surface rounded-xl shadow-panel overflow-hidden flex flex-col">` holding a `data-tauri-drag-region` header strip and the existing `settingsScrollRef` scroller with `max-w-[720px] mx-auto px-8 pt-6 pb-12 gap-8`. Move `<Footer />` into the sidebar. Keep `WhatsNewGate`, `AccessibilityPermissions`, `SecureInputWarning`, `dir={direction}`, scroll reset, onboarding branches untouched. Update the Toaster `classNames` (6.11) and the onboarding-preview exit button to `Button variant="secondary"`. 8. `src/components/Sidebar.tsx`: per 6.1 (button elements, `aria-current`, 16px icons, drag region at top, footer slot). `SECTIONS_CONFIG` and filtering logic unchanged. 9. `src/components/footer/Footer.tsx`: vertical/compact layout for the sidebar; `ModelSelector` and `UpdateChecker` restyled to ghost/caption (check `src/components/model-selector/` and `src/components/update-checker/` for their own pink classes).

**Phase 3 — Primitives (`src/components/ui/`)** — keep every prop/variant name 10. `Button.tsx` (6.6, add `size="icon"`), `ResetButton.tsx`. 11. `Input.tsx`, `Textarea.tsx` (6.7), `Select.tsx` + `.app-select__*` (6.9), `Dropdown.tsx` (6.9). 12. `ToggleSwitch.tsx` (6.8), `Slider.tsx`, `Tooltip.tsx`, `Badge.tsx`, `Alert.tsx` (6.12), `Dialog.tsx` (6.10). 13. `SettingsGroup.tsx`, `SettingContainer.tsx` (6.4), `TextDisplay.tsx`, `PathDisplay.tsx` (mono, `bg-surface-sunken rounded-sm`), `AudioPlayer.tsx` (6.5). 14. New `PageHeader.tsx`, `StatCard.tsx`; export from `ui/index.ts`.

**Phase 4 — Screens & sweep** 15. Each section component (`settings/general/GeneralSettings.tsx`, `history/HistorySettings.tsx`, `models/ModelsSettings.tsx`, `advanced/`, `post-processing/`, `debug/`, `about/`) gets a `<PageHeader title={t(labelKey)} />` at the top and `gap-8` between groups. History rows per 6.5 (hover-revealed actions). 16. Sweep legacy classes across `src/` (~42 files): `grep -rn "logo-primary\|background-ui\|mid-gray\|bg-background\b" src`. Mapping: `bg-background` → `bg-surface`; `text-mid-gray` → `text-text-secondary`; `text-text/60` → `text-text-secondary`; `text-text/40` → `text-text-tertiary`; `border-mid-gray/20` → `border-border`; `border-mid-gray/80` → `border-border-strong`; `bg-mid-gray/10` → `bg-fill-hover` (or `bg-surface-sunken` for fields); `bg-mid-gray/20` → `bg-fill-active`; `hover:border-logo-primary` → drop; `bg-logo-primary/*` (selected) → `bg-fill-selected`; `bg-background-ui` → `bg-accent`; `ring-logo-primary` → focus-visible outline; `text-red-*`/`bg-red-*` → `text-error`/`bg-error`; `yellow-*` → `warning`. Logo components (`icons/HandyTextLogo`, `logo-primary` / `logo-stroke` CSS classes) keep using brand tokens. 17. `src/components/onboarding/*`, `whats-new/*`, `model-selector/*`, `update-checker/*`, `AccessibilityPermissions.tsx`, `SecureInputWarning.tsx`: apply the same mapping. 18. Remove the legacy aliases from `theme.css` / `@theme inline` once grep is clean; fix whatever `bun run build` flags.

**Phase 5 — Overlay** 19. `src/overlay/RecordingOverlay.css` per 6.13 (tokens only, geometry vars untouched); verify in `overlay.rs` that the window footprint still contains the card + shadow.

**Phase 6 — QA** 20. Matrix: {macOS, Windows 11, Windows 10, Linux} × {light, dark, system} × {en, ar (RTL), ja, ru}. Check: drag strip works and buttons inside it click; traffic lights don't overlap the logo; Mica/vibrancy only behind the sidebar; toggles/focus visible with keyboard; reduced motion; Playwright tests (update selectors if any relied on `div` sidebar items — they become `button`s); translations script passes (only new keys: stat labels, if StatCard is shipped with data).
