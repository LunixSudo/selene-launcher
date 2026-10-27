import { en } from "./en";
import { ru } from "./ru";

export type Messages = typeof ru;
export type Locale = "ru" | "en";

type Leaves<T, Prefix extends string = ""> = {
  [K in keyof T & string]: T[K] extends string ? `${Prefix}${K}` : Leaves<T[K], `${Prefix}${K}.`>;
}[keyof T & string];

export type MessageKey = Leaves<Messages>;

const STORAGE_KEY = "locale";
const catalogs: Record<Locale, Messages> = { ru, en };

let current = $state<Locale>(initialLocale());

function initialLocale(): Locale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "ru" || saved === "en") {
      return saved;
    }
  } catch {
    // Storage can be unavailable; fall back to the system language.
  }
  return navigator.language.toLowerCase().startsWith("ru") ? "ru" : "en";
}

export function locale(): Locale {
  return current;
}

export function setLocale(next: Locale) {
  current = next;
  document.documentElement.lang = next;
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    // Not remembering the choice is fine.
  }
}

export function t(key: MessageKey, params: Record<string, string | number> = {}): string {
  let node: unknown = catalogs[current];
  for (const part of key.split(".")) {
    node = (node as Record<string, unknown>)[part];
  }
  return (node as string).replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}
