import { Effect, getCurrentWindow } from "@tauri-apps/api/window";

export type Theme = "system" | "light" | "dark";
export type Style = "glass" | "mica" | "solid";
export type Accent = "violet" | "blue" | "green" | "pink" | "orange";

export const THEMES: Theme[] = ["system", "light", "dark"];
export const STYLES: Style[] = ["glass", "mica", "solid"];
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
}

export const settings = $state<Settings>(load());

function load(): Settings {
  const defaults: Settings = { theme: "system", style: "mica", accent: "violet" };
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}") as Partial<Settings>;
    return {
      theme: THEMES.includes(saved.theme as Theme) ? (saved.theme as Theme) : defaults.theme,
      style: STYLES.includes(saved.style as Style) ? (saved.style as Style) : defaults.style,
      accent: saved.accent && saved.accent in ACCENTS ? saved.accent : defaults.accent,
    };
  } catch {
    return defaults;
  }
}

// Call from an effect so it reruns whenever a setting changes.
export function applySettings() {
  const root = document.documentElement;
  root.dataset.theme = settings.theme;
  root.dataset.style = settings.style;
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
