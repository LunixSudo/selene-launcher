import { locale } from "$lib/i18n/index.svelte";

// Modrinth sends tag slugs and CurseForge English names; neither is translated.
const RUSSIAN: Record<string, string> = {
  adventure: "Приключения",
  cursed: "Странное",
  decoration: "Декор",
  economy: "Экономика",
  equipment: "Снаряжение",
  food: "Еда",
  "game-mechanics": "Механики игры",
  library: "Библиотеки",
  magic: "Магия",
  management: "Управление",
  minigame: "Мини-игры",
  mobs: "Мобы",
  optimization: "Оптимизация",
  social: "Общение",
  storage: "Хранение",
  technology: "Техника",
  transportation: "Транспорт",
  utility: "Утилиты",
  worldgen: "Генерация мира",
  combat: "Бой",
  modded: "Для модов",
  realistic: "Реалистичные",
  simplistic: "Простые",
  themed: "Тематические",
  tweaks: "Улучшения",
  "vanilla-like": "Как в ванилле",
  cartoon: "Мультяшные",
  fantasy: "Фэнтези",
  "semi-realistic": "Полуреалистичные",
  "API and Library": "API и библиотеки",
  Addons: "Дополнения",
  "Adventure and RPG": "Приключения и RPG",
  "Armor, Tools, and Weapons": "Броня, инструменты и оружие",
  "Bug Fixes": "Исправления",
  Cosmetic: "Косметика",
  Education: "Обучение",
  Food: "Еда",
  Horror: "Хоррор",
  Magic: "Магия",
  "Map and Information": "Карты и информация",
  Miscellaneous: "Разное",
  Mobs: "Мобы",
  Performance: "Оптимизация",
  Redstone: "Редстоун",
  "Server Utility": "Для серверов",
  Storage: "Хранение",
  Technology: "Техника",
  "Utility & QoL": "Утилиты и удобство",
  "World Gen": "Генерация мира",
  Realistic: "Реалистичные",
  Fantasy: "Фэнтези",
  Vanilla: "Как в ванилле",
  Medieval: "Средневековье",
  Modern: "Современные",
  Traditional: "Классические",
  Animated: "Анимированные",
  "Photo Realistic": "Фотореализм",
  "Mod Support": "Для модов",
  "Data Packs": "Датапаки",
  "Font Packs": "Шрифты",
  Steampunk: "Стимпанк",
};

export function categoryName(name: string): string {
  if (locale() === "ru" && name in RUSSIAN) {
    return RUSSIAN[name];
  }
  return name
    .split("-")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}
