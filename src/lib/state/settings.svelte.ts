import { Effect, getCurrentWindow } from "@tauri-apps/api/window";
import { SHADER_BACKDROPS, type ShaderBackdrop } from "$lib/backdrops";

export type Theme = "system" | "light" | "dark";
export type Style = "glass" | "mica" | "solid";
export type Accent = "violet" | "blue" | "green" | "pink" | "orange";
export type Font = "rounded" | "modern" | "rubik" | "system";
export type AfterLaunch = "keep" | "minimize" | "close";
export type SortOrder = "lastPlayed" | "name" | "playTime" | "created";
export type Backdrop = "none" | "stars" | ShaderBackdrop;
/** An instance's own background: the launcher's, its cover, or any of the animated ones. */
export type InstanceBackdrop = "app" | "cover" | Exclude<Backdrop, "none">;
export type SkinAnimation = "idle" | "walk" | "run" | "fly" | "wave";

export const THEMES: Theme[] = ["system", "light", "dark"];
export const STYLES: Style[] = ["glass", "mica", "solid"];
export const FONTS: Font[] = ["rounded", "modern", "rubik", "system"];
export const AFTER_LAUNCH: AfterLaunch[] = ["keep", "minimize", "close"];
export const SORT_ORDERS: SortOrder[] = ["lastPlayed", "name", "playTime", "created"];
export const BACKDROPS: Backdrop[] = ["none", "stars", ...SHADER_BACKDROPS];
export const INSTANCE_BACKDROPS: InstanceBackdrop[] = [
  "app",
  "cover",
  "stars",
  ...SHADER_BACKDROPS,
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
  /** Looks the user saved to switch between quickly. */
  presets: StylePreset[];
  /** Lists fade out at the edges where they scroll on. */
  scrollFade: boolean;
}

/** Everything that makes up how the launcher looks. */
export type Look = Pick<
  Settings,
  "theme" | "style" | "accent" | "font" | "backdrop" | "motion" | "clickSound"
>;

export interface StylePreset {
  name: string;
  look: Look;
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
  presets: [],
  scrollFade: true,
};

/** The two looks offered on the first start and kept in the settings. Colours stay as set. */
export const FAST: Partial<Look> = {
  style: "solid",
  backdrop: "none",
  motion: false,
  clickSound: false,
};

export const BEAUTIFUL: Partial<Look> = {
  style: "mica",
  backdrop: "aurora",
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
  const checkLook = (look: Partial<Look>): Look => ({
    theme: oneOf(look.theme, THEMES, DEFAULTS.theme),
    style: oneOf(look.style, STYLES, DEFAULTS.style),
    accent: oneOf(look.accent, Object.keys(ACCENTS) as Accent[], DEFAULTS.accent),
    font: oneOf(look.font, FONTS, DEFAULTS.font),
    backdrop: oneOf(look.backdrop, BACKDROPS, DEFAULTS.backdrop),
    motion: oneOf(look.motion, [true, false], DEFAULTS.motion),
    clickSound: oneOf(look.clickSound, [true, false], DEFAULTS.clickSound),
  });
  return {
    ...checkLook(saved),
    afterLaunch: oneOf(saved.afterLaunch, AFTER_LAUNCH, DEFAULTS.afterLaunch),
    restoreAfterGame: oneOf(saved.restoreAfterGame, [true, false], DEFAULTS.restoreAfterGame),
    sort: oneOf(saved.sort, SORT_ORDERS, DEFAULTS.sort),
    collapsedGroups: Array.isArray(saved.collapsedGroups)
      ? saved.collapsedGroups.filter((group) => typeof group === "string")
      : DEFAULTS.collapsedGroups,
    notifications: oneOf(saved.notifications, [true, false], DEFAULTS.notifications),
    checkUpdates: oneOf(saved.checkUpdates, [true, false], DEFAULTS.checkUpdates),
    discordStatus: oneOf(saved.discordStatus, [true, false], DEFAULTS.discordStatus),
    instanceBackdrops: Object.fromEntries(
      Object.entries(saved.instanceBackdrops ?? {}).filter(([, backdrop]) =>
        INSTANCE_BACKDROPS.includes(backdrop),
      ),
    ),
    skinAnimation: oneOf(saved.skinAnimation, SKIN_ANIMATIONS, DEFAULTS.skinAnimation),
    setupDone: oneOf(saved.setupDone, [true, false], DEFAULTS.setupDone),
    presets: Array.isArray(saved.presets)
      ? saved.presets
          .filter((preset) => typeof preset?.name === "string" && preset.look)
          .map((preset) => ({ name: preset.name, look: checkLook(preset.look) }))
      : DEFAULTS.presets,
    scrollFade: oneOf(saved.scrollFade, [true, false], DEFAULTS.scrollFade),
  };
}

export function currentLook(from: Look = settings): Look {
  const { theme, style, accent, font, backdrop, motion, clickSound } = from;
  return { theme, style, accent, font, backdrop, motion, clickSound };
}

/** Whether the settings already look the way `look` describes. */
export function matchesLook(look: Partial<Look>): boolean {
  return Object.entries(look).every(([key, value]) => settings[key as keyof Look] === value);
}

// Call from an effect so it reruns whenever a setting changes.
export function applySettings() {
  const root = document.documentElement;
  root.dataset.theme = settings.theme;
  root.dataset.style = settings.style;
  root.dataset.font = settings.font;
  root.dataset.motion = settings.motion ? "on" : "off";
  root.dataset.scrollFade = settings.scrollFade ? "on" : "off";
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
