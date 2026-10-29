import { de } from "./de";
import { en } from "./en";
import { es } from "./es";
import { pl } from "./pl";
import { ptBR } from "./pt-BR";
import { ru } from "./ru";
import { tr } from "./tr";
import { uk } from "./uk";
import { zhCN } from "./zh-CN";

export type Messages = typeof ru;
export type Locale = "ru" | "en" | "uk" | "de" | "es" | "pt-BR" | "pl" | "tr" | "zh-CN";

/** Each language under its own name, the way the language list shows it. */
export const LANGUAGES: { value: Locale; label: string }[] = [
  { value: "ru", label: "Русский" },
  { value: "en", label: "English" },
  { value: "uk", label: "Українська" },
  { value: "de", label: "Deutsch" },
  { value: "es", label: "Español" },
  { value: "pt-BR", label: "Português (Brasil)" },
  { value: "pl", label: "Polski" },
  { value: "tr", label: "Türkçe" },
  { value: "zh-CN", label: "简体中文" },
];

/** Languages the team writes itself; the others are machine translations. */
export const CHECKED_LOCALES: Locale[] = ["ru", "en"];

type Leaves<T, Prefix extends string = ""> = {
  [K in keyof T & string]: T[K] extends string ? `${Prefix}${K}` : Leaves<T[K], `${Prefix}${K}.`>;
}[keyof T & string];

export type MessageKey = Leaves<Messages>;

const STORAGE_KEY = "locale";
const catalogs: Record<Locale, Messages> = {
  ru,
  en,
  uk,
  de,
  es,
  "pt-BR": ptBR,
  pl,
  tr,
  "zh-CN": zhCN,
};

let current = $state<Locale>(initialLocale());

function initialLocale(): Locale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved && saved in catalogs) {
      return saved as Locale;
    }
  } catch {
    // Storage can be unavailable; fall back to the system language.
  }
  for (const wanted of navigator.languages) {
    const match = matchLocale(wanted);
    if (match) {
      return match;
    }
  }
  return "en";
}

/** `pt-PT` still gets Brazilian Portuguese and `zh-TW` simplified Chinese: close beats English. */
function matchLocale(tag: string): Locale | null {
  const exact = LANGUAGES.find((option) => option.value.toLowerCase() === tag.toLowerCase());
  const language = tag.split("-")[0].toLowerCase();
  const near = LANGUAGES.find((option) => option.value.split("-")[0].toLowerCase() === language);
  return (exact ?? near)?.value ?? null;
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
