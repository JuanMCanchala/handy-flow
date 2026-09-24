import React, { lazy } from "react";
import { useTranslation } from "react-i18next";
import {
  Cog,
  Cpu,
  FlaskConical,
  History,
  Home,
  Info,
  Languages,
  MessageCircleQuestion,
  NotebookPen,
  Palette,
  Sparkles,
  Type,
  Wand2,
} from "lucide-react";
import VoxaTextLogo from "./icons/VoxaTextLogo";
import VoxaWaveformIcon from "./icons/VoxaWaveformIcon";
import Footer from "./footer";
import { useSettings } from "../hooks/useSettings";

// Each sidebar section's settings UI is code-split into its own chunk:
// only the active section's JS/CSS is fetched, keeping the initial bundle
// (and WebView2 parse/compile cost) small. `DebugSettings` is loaded eagerly
// via the barrel in `App.tsx` for the onboarding-preview flow, so it is
// exempt here.
const HomeSettings = lazy(() =>
  import("./home/HomeSettings").then((m) => ({ default: m.HomeSettings })),
);
const GeneralSettings = lazy(() =>
  import("./settings/general/GeneralSettings").then((m) => ({
    default: m.GeneralSettings,
  })),
);
const HistorySettings = lazy(() =>
  import("./settings/history/HistorySettings").then((m) => ({
    default: m.HistorySettings,
  })),
);
const ScratchpadSettings = lazy(() =>
  import("./scratchpad").then((m) => ({ default: m.ScratchpadSettings })),
);
const ModelsSettings = lazy(() =>
  import("./settings/models/ModelsSettings").then((m) => ({
    default: m.ModelsSettings,
  })),
);
const AdvancedSettings = lazy(() =>
  import("./settings/advanced/AdvancedSettings").then((m) => ({
    default: m.AdvancedSettings,
  })),
);
const SnippetsSettings = lazy(() =>
  import("./settings/snippets/SnippetsSettings").then((m) => ({
    default: m.SnippetsSettings,
  })),
);
const TransformsSettings = lazy(() =>
  import("./settings/transforms/TransformsSettings").then((m) => ({
    default: m.TransformsSettings,
  })),
);
const PostProcessingSettings = lazy(() =>
  import("./settings/post-processing/PostProcessingSettings").then((m) => ({
    default: m.PostProcessingSettings,
  })),
);
const StyleSettings = lazy(() =>
  import("./settings/StyleSettings").then((m) => ({
    default: m.StyleSettings,
  })),
);
const LiveTranslateSettings = lazy(() =>
  import("./settings/live-translate/LiveTranslateSettings").then((m) => ({
    default: m.LiveTranslateSettings,
  })),
);
const CopilotSettings = lazy(() =>
  import("./settings/copilot/CopilotSettings").then((m) => ({
    default: m.CopilotSettings,
  })),
);
const AboutSettings = lazy(() =>
  import("./settings/about/AboutSettings").then((m) => ({
    default: m.AboutSettings,
  })),
);
// DebugSettings stays a normal (non-lazy) import: App.tsx already renders it
// directly for the onboarding preview flow, so it is on the eager path
// regardless.
import { DebugSettings } from "./settings";

export type SidebarSection = keyof typeof SECTIONS_CONFIG;

interface IconProps {
  width?: number | string;
  height?: number | string;
  size?: number | string;
  className?: string;
  [key: string]: any;
}

interface SectionConfig {
  labelKey: string;
  icon: React.ComponentType<IconProps>;
  component: React.ComponentType;
  enabled: (settings: any) => boolean;
}

export const SECTIONS_CONFIG = {
  home: {
    labelKey: "sidebar.home",
    icon: Home,
    component: HomeSettings,
    enabled: () => true,
  },
  general: {
    labelKey: "sidebar.general",
    icon: VoxaWaveformIcon,
    component: GeneralSettings,
    enabled: () => true,
  },
  history: {
    labelKey: "sidebar.history",
    icon: History,
    component: HistorySettings,
    enabled: () => true,
  },
  scratchpad: {
    labelKey: "sidebar.scratchpad",
    icon: NotebookPen,
    component: ScratchpadSettings,
    enabled: () => true,
  },
  models: {
    labelKey: "sidebar.models",
    icon: Cpu,
    component: ModelsSettings,
    enabled: () => true,
  },
  advanced: {
    labelKey: "sidebar.advanced",
    icon: Cog,
    component: AdvancedSettings,
    enabled: () => true,
  },
  snippets: {
    labelKey: "sidebar.snippets",
    icon: Type,
    component: SnippetsSettings,
    enabled: () => true,
  },
  transforms: {
    labelKey: "sidebar.transforms",
    icon: Wand2,
    component: TransformsSettings,
    enabled: () => true,
  },
  postprocessing: {
    labelKey: "sidebar.postProcessing",
    icon: Sparkles,
    component: PostProcessingSettings,
    enabled: (settings) => settings?.post_process_enabled ?? false,
  },
  style: {
    labelKey: "sidebar.style",
    icon: Palette,
    component: StyleSettings,
    enabled: () => true,
  },
  liveTranslate: {
    labelKey: "sidebar.liveTranslate",
    icon: Languages,
    component: LiveTranslateSettings,
    enabled: () => true,
  },
  copilot: {
    labelKey: "sidebar.copilot",
    icon: MessageCircleQuestion,
    component: CopilotSettings,
    enabled: () => true,
  },
  debug: {
    labelKey: "sidebar.debug",
    icon: FlaskConical,
    component: DebugSettings,
    enabled: (settings) => settings?.debug_mode ?? false,
  },
  about: {
    labelKey: "sidebar.about",
    icon: Info,
    component: AboutSettings,
    enabled: () => true,
  },
} as const satisfies Record<string, SectionConfig>;

interface SidebarProps {
  activeSection: SidebarSection;
  onSectionChange: (section: SidebarSection) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeSection,
  onSectionChange,
}) => {
  const { t } = useTranslation();
  const { settings } = useSettings();

  const availableSections = Object.entries(SECTIONS_CONFIG)
    .filter(([_, config]) => config.enabled(settings))
    .map(([id, config]) => ({ id: id as SidebarSection, ...config }));

  return (
    <div
      className="w-[var(--sidebar-w)] shrink-0 h-full flex flex-col bg-transparent pt-[var(--titlebar-h)] px-3 pb-3"
      data-tauri-drag-region
    >
      <VoxaTextLogo width={88} className="ms-2 mb-6" />
      <div className="flex flex-col w-full gap-0.5">
        {availableSections.map((section) => {
          const Icon = section.icon;
          const isActive = activeSection === section.id;

          return (
            <button
              key={section.id}
              type="button"
              aria-current={isActive ? "page" : undefined}
              className={`h-8 w-full flex items-center gap-2.5 px-2.5 rounded-md text-body font-medium transition-colors duration-[var(--dur-fast)] cursor-default ${
                isActive
                  ? "bg-fill-active text-text shadow-[inset_0_0_0_1px_var(--color-border)]"
                  : "text-text-secondary hover:bg-fill-hover hover:text-text"
              }`}
              onClick={() => onSectionChange(section.id)}
            >
              <Icon
                size={16}
                strokeWidth={1.75}
                className={`shrink-0 ${isActive ? "text-text" : "text-text-tertiary"}`}
              />
              <p className="truncate" title={t(section.labelKey)}>
                {t(section.labelKey)}
              </p>
            </button>
          );
        })}
      </div>
      <div className="mt-auto border-t border-border pt-3">
        <Footer />
      </div>
    </div>
  );
};
