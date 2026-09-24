import React from "react";
import ReactDOM from "react-dom/client";
import { platform, version } from "@tauri-apps/plugin-os";
import App from "./App";
import { installCompatShims } from "./lib/compat";
import {
  applyTheme,
  getStoredTheme,
  syncThemeFromSettings,
} from "./lib/utils/theme";

installCompatShims();

// Set platform before render so CSS can scope per-platform (e.g. scrollbar styles)
const currentPlatform = platform();
document.documentElement.dataset.platform = currentPlatform;

// Native window effects (macOS Sidebar vibrancy, Windows 11 22H2+ Mica) make
// the window background transparent behind the sidebar; everywhere else the
// canvas must stay opaque. See docs/design/macos-style.md section 4.
const isVibrancyCapable = (() => {
  if (currentPlatform === "macos") return true;
  if (currentPlatform === "windows") {
    // Windows build 22000 is the first Windows 11 build (Mica support).
    const build = parseInt(version().split(".")[2] ?? "", 10);
    return Number.isFinite(build) && build >= 22000;
  }
  return false;
})();
if (isVibrancyCapable) {
  document.documentElement.dataset.vibrancy = "on";
}

// Apply the last-known theme synchronously before render to avoid a flash of
// the wrong palette, then reconcile with the persisted setting once it loads.
applyTheme(getStoredTheme());
syncThemeFromSettings();

// Initialize i18n
import "./i18n";

// Initialize model store (loads models and sets up event listeners)
import { useModelStore } from "./stores/modelStore";
useModelStore.getState().initialize();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
