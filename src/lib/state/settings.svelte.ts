import { Effect, getCurrentWindow } from "@tauri-apps/api/window";

export type Theme = "system" | "light" | "dark";
export type Style = "glass" | "mica" | "solid";
export type Accent = "violet" | "blue" | "green" | "pink" | "orange";
export type Font = "rounded" | "modern" | "rubik" | "system";
export type AfterLaunch = "keep" | "minimize" | "close";
export type SortOrder = "lastPlayed" | "name" | "playTime" | "created";
export type Backdrop = "none" | "halo" | "rays" | "stars" | "aurora";
/** An instance's own background: the launcher's, its cover, or any of the animated ones. */
export type InstanceBackdrop = "app" | "cover" | Exclude<Backdrop, "none">;
export type SkinAnimation = "idle" | "walk" | "run" | "fly" | "wave";

export const THEMES: Theme[] = ["system", "light", "dark"];
export const STYLES: Style[] = ["glass", "mica", "solid"];
export const FONTS: Font[] = ["rounded", "modern", "rubik", "system"];
export const AFTER_LAUNCH: AfterLaunch[] = ["keep", "minimize", "close"];
export const SORT_ORDERS: SortOrder[] = ["lastPlayed", "name", "playTime", "created"];
export const BACKDROPS: Backdrop[] = ["none", "halo", "rays", "stars", "aurora"];
export const INSTANCE_BACKDROPS: InstanceBackdrop[] = [
  "app",
  "cover",
  "halo",
  "rays",
  "stars",
  "aurora",
];
export const SKIN_ANIMATIONS: SkinAnimation[] = ["idle", "walk", "run", "fly", "wave"];
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
  notifications: boolean;
  checkUpdates: boolean;
  discordStatus: boolean;
  /** An animated background behind the whole window. */
  backdrop: Backdrop;
  /** Page transitions, hover lifts and the glowing play button. */
  motion: boolean;
  instanceBackdrops: Record<string, InstanceBackdrop>;
  /** A soft click when a button is pressed. */
  clickSound: boolean;
  skinAnimation: SkinAnimation;
  /** Whether the first-start question about looks or speed was answered. */
  setupDone: boolean;
}

const DEFAULTS: Settings = {
  theme: "system",
  style: "solid",
  accent: "violet",
  font: "rounded",
  afterLaunch: "close",
  restoreAfterGame: true,
  sort: "lastPlayed",
  collapsedGroups: [],
  notifications: true,
  checkUpdates: true,
  discordStatus: true,
  backdrop: "none",
  motion: false,
  instanceBackdrops: {},
  clickSound: false,
  skinAnimation: "walk",
  setupDone: false,
};

/** What "make it beautiful" on the first start turns on. */
export const BEAUTIFUL: Partial<Settings> = {
  style: "glass",
  backdrop: "halo",
  motion: true,
  clickSound: true,
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
    notifications: oneOf(saved.notifications, [true, false], DEFAULTS.notifications),
    checkUpdates: oneOf(saved.checkUpdates, [true, false], DEFAULTS.checkUpdates),
    discordStatus: oneOf(saved.discordStatus, [true, false], DEFAULTS.discordStatus),
    backdrop: oneOf(saved.backdrop, BACKDROPS, DEFAULTS.backdrop),
    motion: oneOf(saved.motion, [true, false], DEFAULTS.motion),
    instanceBackdrops: Object.fromEntries(
      Object.entries(saved.instanceBackdrops ?? {}).filter(([, backdrop]) =>
        INSTANCE_BACKDROPS.includes(backdrop),
      ),
    ),
    clickSound: oneOf(saved.clickSound, [true, false], DEFAULTS.clickSound),
    skinAnimation: oneOf(saved.skinAnimation, SKIN_ANIMATIONS, DEFAULTS.skinAnimation),
    setupDone: oneOf(saved.setupDone, [true, false], DEFAULTS.setupDone),
  };
}

// Call from an effect so it reruns whenever a setting changes.
export function applySettings() {
  const root = document.documentElement;
  root.dataset.theme = settings.theme;
  root.dataset.style = settings.style;
  root.dataset.font = settings.font;
  root.dataset.motion = settings.motion ? "on" : "off";
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
