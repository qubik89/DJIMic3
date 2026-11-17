import { writable, derived } from "svelte/store";
import { normalizeLocale, translate } from "./translations.js";
import { helpFor } from "./settingHelp.js";

export const LOCALE_KEY = "djimic.locale";

// Saved choice first, then the system language, then English.
let initialLocale = "en";
try {
  initialLocale =
    normalizeLocale(localStorage.getItem(LOCALE_KEY)) ??
    normalizeLocale(navigator.language) ??
    "en";
} catch {
  initialLocale = normalizeLocale(globalThis.navigator?.language) ?? "en";
}

export const locale = writable(initialLocale);
/** `$t("English text", { name })` → translated text. */
export const t = derived(locale, (value) => (key, params) => translate(value, key, params));
/** `$help(settingId)` → the ⓘ explanation in the current language. */
export const help = derived(locale, (value) => (id) => helpFor(value, id));

export function selectLocale(value) {
  const next = normalizeLocale(value) ?? "en";
  locale.set(next);
  try {
    localStorage.setItem(LOCALE_KEY, next);
  } catch {
    // Storage unavailable: the choice lasts for this session only.
  }
}
