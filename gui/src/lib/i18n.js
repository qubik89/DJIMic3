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
/**
 * Messages the backend sends back in English (device, import, download and
 * Wi-Fi errors), matched to translatable keys. Anything unknown (an OS error,
 * say) is shown as it came.
 */
const ERRORS = [
  [/^device not found: (.+)$/, "Device not found: {detail}"],
  [/^permission denied opening device/, "Permission denied opening the device (see the USB setup)."],
  [/^unknown setting: (.+)$/, "Unknown setting: {detail}"],
  [/^invalid value "(.*)" for setting "(.*)"$/, "Invalid value {detail} for {extra}"],
  [/^usb error: (.+)$/, "USB error: {detail}"],
  [/^device rejected the command \(code (0x[0-9a-f]+)\)$/, "The device rejected the command (code {detail})."],
  [/^device did not acknowledge the command$/, "The device didn't acknowledge the command."],
  [/^device disconnected$/, "Device disconnected."],
  [/^No import folder$/, "Choose an import folder first."],
  [/^An import is already running$/, "An import is already running."],
  [/^Wait for the import to finish$/, "Wait for the import to finish."],
  [/^Wait for the download to finish$/, "Wait for the download to finish."],
  [/^Not a connected transmitter$/, "That transmitter is no longer connected."],
  [/^The transmitter is not connected$/, "That transmitter is no longer connected."],
  [/^Unexpected drive path$/, "Unexpected drive path."],
  [/^already connecting$/, "A connection is already in progress."],
  [/^(.+) is not on a connected transmitter$/, "{detail} is not on a connected transmitter."],
  [/^not on a connected transmitter$/, "That transmitter is no longer connected."],
  [/^not imported yet$/, "Not imported yet."],
  [/^read (\d+) of (\d+) bytes$/, "Only {detail} of {extra} bytes could be read."],
  [/^the copy doesn't match the original$/, "The copy doesn't match the original."],
  [/^the downloaded file is incomplete$/, "The downloaded file is incomplete."],
  [/^connection closed at (\d+) of (\d+) bytes$/, "The connection dropped at {detail} of {extra} bytes."],
  [/^server reports (\d+) bytes, expected (\d+)$/, "The transmitter reports {detail} bytes instead of {extra}."],
  [/^HTTP (\d+)$/, "The transmitter answered with HTTP error {detail}."],
  [/^control channel: (.+)$/, "Couldn't open the control session: {detail}"],
  [/^the transmitter didn't start its file server$/, "The transmitter didn't start its file server."],
  [/^file list: (.+)$/, "Couldn't read the list of recordings: {detail}"],
  [/^cancelled$/, "Cancelled."],
];

/** `$tError(message)` → a backend error in the current language. */
export const tError = derived(locale, (value) => (message) => {
  const text = String(message ?? "").replace(/^Error: /, "");
  for (const [re, key] of ERRORS) {
    const m = text.match(re);
    if (m) return translate(value, key, { detail: m[1] ?? "", extra: m[2] ?? "" });
  }
  return text;
});

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
