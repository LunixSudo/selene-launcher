import { Effect, getCurrentWindow } from "@tauri-apps/api/window";

export type Theme = "system" | "light" | "dark";
export type Style = "glass" | "mica" | "solid";
export type Accent = "violet" | "blue" | "green" | "pink" | "orange";
export type Font = "rounded" | "modern" | "rubik" | "system";
export type AfterLaunch = "keep" | "minimize" | "close";
export type SortOrder = "lastPlayed" | "name" | "playTime" | "created";

export const THEMES: Theme[] = ["system", "light", "dark"];
export const STYLES: Style[] = ["glass", "mica", "solid"];
export const FONTS: Font[] = ["rounded", "modern", "rubik", "system"];
export const AFTER_LAUNCH: AfterLaunch[] = ["keep", "minimize", "close"];
export const SORT_ORDERS: SortOrder[] = ["lastPlayed", "name", "playTime", "created"];
export const ACCENTS: Record<Accent, string> = {
  violet: "#6b5bff",
  blue: "#0f6cbd",
  green: "#1f8a50",
  pink: "#d6337d",
  orange: "#d86613",
};

const STORAGE_KEY = "settings";

interface Settings {
  theme: Theme;
  style: Style;
  accent: Accent;
  font: Font;
  afterLaunch: AfterLaunch;
  restoreAfterGame: boolean;
  sort: SortOrder;
  collapsedGroups: string[];
}

const DEFAULTS: Settings = {
  theme: "system",
  style: "mica",
  accent: "violet",
  font: "rounded",
  afterLaunch: "minimize",
  restoreAfterGame: true,
  sort: "lastPlayed",
  collapsedGroups: [],
};

export const settings = $state<Settings>(load());

function load(): Settings {
  let saved: Partial<Settings> = {};
  try {
    saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}");
  } catch {
    // Start from the defaults.
  }
  const oneOf = <T>(value: unknown, allowed: readonly T[], fallback: T): T =>
    allowed.includes(value as T) ? (value as T) : fallback;
  return {
    theme: oneOf(saved.theme, THEMES, DEFAULTS.theme),
    style: oneOf(saved.style, STYLES, DEFAULTS.style),
    accent: oneOf(saved.accent, Object.keys(ACCENTS) as Accent[], DEFAULTS.accent),
    font: oneOf(saved.font, FONTS, DEFAULTS.font),
    afterLaunch: oneOf(saved.afterLaunch, AFTER_LAUNCH, DEFAULTS.afterLaunch),
    restoreAfterGame: oneOf(saved.restoreAfterGame, [true, false], DEFAULTS.restoreAfterGame),
    sort: oneOf(saved.sort, SORT_ORDERS, DEFAULTS.sort),
    collapsedGroups: Array.isArray(saved.collapsedGroups)
      ? saved.collapsedGroups.filter((group) => typeof group === "string")
      : DEFAULTS.collapsedGroups,
  };
}

// Call from an effect so it reruns whenever a setting changes.
export function applySettings() {
  const root = document.documentElement;
  root.dataset.theme = settings.theme;
  root.dataset.style = settings.style;
  root.dataset.font = settings.font;
  root.style.setProperty("--accent", ACCENTS[settings.accent]);

  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Not remembering the choice is fine.
  }

  const appWindow = getCurrentWindow();
  void appWindow.setTheme(settings.theme === "system" ? null : settings.theme).catch(() => {});
  const effect = { glass: Effect.Acrylic, mica: Effect.Mica, solid: null }[settings.style];
  void (effect ? appWindow.setEffects({ effects: [effect] }) : appWindow.clearEffects()).catch(
    () => {},
  );
}
