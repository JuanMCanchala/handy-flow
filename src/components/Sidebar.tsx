import React from "react";
import { useTranslation } from "react-i18next";
import {
  Cog,
  Cpu,
  FileText,
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
import { HomeSettings } from "./home/HomeSettings";
import {
  AboutSettings,
  AdvancedSettings,
  CopilotSettings,
  DebugSettings,
  GeneralSettings,
  HistorySettings,
  LiveTranslateSettings,
  ModelsSettings,
  PostProcessingSettings,
  SnippetsSettings,
  StyleSettings,
  TemplatesSettings,
  TransformsSettings,
} from "./settings";
import { ScratchpadSettings } from "./scratchpad";

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
  templates: {
    labelKey: "sidebar.templates",
    icon: FileText,
    component: TemplatesSettings,
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
