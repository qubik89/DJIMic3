// Ko-fi reminder. The "Buy me a coffee" button is always there. The card
// shows up on its own once the app has proven itself (a receiver with at
// least one transmitter linked), at most a few times, and never again once
// the page has been opened or the user says so. Its state lives in
// localStorage: nothing is counted or sent anywhere else.
import { invoke } from "@tauri-apps/api/core";

const KEY = "djimic.support";
/** How long the window must be in front with a transmitter linked. */
export const DELAY_MS = 30_000;
/** Wait between two cards. */
export const SNOOZE_MS = 30 * 24 * 60 * 60 * 1000;
/** Cards shown in total, ever. */
export const MAX_SHOWS = 3;

/** Cards `shown`, no card before `snoozeUntil`, `done` for good. */
const EMPTY = { shown: 0, snoozeUntil: 0, done: false };

export function load() {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? "null");
    return { ...EMPTY, ...(saved && typeof saved === "object" ? saved : {}) };
  } catch {
    return { ...EMPTY };
  }
}

export function save(state) {
  try {
    localStorage.setItem(KEY, JSON.stringify(state));
  } catch {
    // Storage unavailable: the reminder state lasts for this session only.
  }
}

/** The receiver is connected and at least one transmitter is linked to it. */
export const working = (status) => !!status?.connected && (status.tx ?? []).some(Boolean);

export const shouldShow = (state, now) => !state.done && state.shown < MAX_SHOWS && now >= state.snoozeUntil;

/** The card is on screen: count it and hold the next one back. */
export const markShown = (state, now) => ({ ...state, shown: state.shown + 1, snoozeUntil: now + SNOOZE_MS });

/** Never show the card again. */
export const stop = (state) => ({ ...state, done: true });

/** Open the Ko-fi page. The app answers with a `support-opened` event. */
export function openSupport() {
  return invoke("open_support");
}
