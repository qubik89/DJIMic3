// Transmitter recordings: the drives found, the import folder and the
// running import. Lives outside the panel so an import keeps reporting
// while the panel is closed. Desktop only (the commands don't exist on
// Android).
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { writable, get } from "svelte/store";

const DEST_KEY = "djimic.recordings.dest";
const KIND_KEY = "djimic.recordings.kind";

const load = (key) => {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
};
const save = (key, value) => {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* session-only */
  }
};

/** Transmitter drives mounted now (see the `recordings` crate). */
export const drives = writable([]);
/** Import folder. */
export const dest = writable(load(DEST_KEY) ?? "");
/** Which files of a take to copy: "original" | "processed" | "both". */
export const kind = writable(["original", "processed", "both"].includes(load(KIND_KEY)) ? load(KIND_KEY) : "both");
/** { running, progress, outcome } for the current or last import. */
export const job = writable({ running: false, progress: null, outcome: null });
/**
 * Wireless connection: phase "idle" | "connecting" | "connected" | "error",
 * the latest backend step ({ step, ... }), the access point's name and the
 * error ({ kind, detail }) when it failed.
 */
export const wireless = writable({ phase: "idle", step: null, ssid: null, error: null });

kind.subscribe((v) => save(KIND_KEY, v));

export function setDest(value) {
  dest.set(value);
  save(DEST_KEY, value);
}

/** Unique id of a take across transmitters. */
export const takeId = (tx, take) => `${tx.id}/${take.key}`;

/** The files of `take` that `kind` asks for (a take without a processed copy gives its original). */
export function filesFor(take, k) {
  if (k === "both") return take.files;
  const want = k === "processed" ? "processed" : "original";
  const match = take.files.filter((f) => f.kind === want || f.kind === "other");
  return match.length > 0 ? match : take.files.filter((f) => f.kind !== "processed");
}

/** A take is new while any of the files `kind` asks for isn't imported. */
export const isNew = (take, k) => filesFor(take, k).some((f) => !f.imported);

export async function scan() {
  const d = get(dest) || null;
  let list = [];
  try {
    list = await invoke("recordings_scan", { dest: d });
  } catch {
    list = [];
  }
  if (get(wireless).phase === "connected") {
    try {
      const tx = await invoke("wireless_transmitter", { dest: d });
      if (tx) list = [...list, tx];
    } catch {
      /* lost: the wireless-lost event resets the state */
    }
  }
  drives.set(list);
}

/** The two-letter region a unit's Wi-Fi should follow (from the system locale). */
export function region() {
  try {
    const loc = new Intl.Locale(Intl.DateTimeFormat().resolvedOptions().locale).maximize();
    if (loc.region && /^[A-Z]{2}$/.test(loc.region)) return loc.region;
  } catch {
    /* fall through */
  }
  return "US";
}

/** Pair with a transmitter over Bluetooth and join its Wi-Fi. */
export async function startWireless() {
  wireless.set({ phase: "connecting", step: { step: "scanning" }, ssid: null, error: null });
  try {
    await invoke("wireless_connect", { country: region() });
    wireless.update((w) => ({ ...w, phase: "connected", step: { step: "ready" } }));
    await scan();
  } catch (e) {
    const err = typeof e === "object" && e ? e : { kind: "session", detail: String(e) };
    wireless.update((w) => (err.kind === "cancelled" ? { ...w, phase: "idle", step: null } : { ...w, phase: "error", error: err }));
  }
}

export const cancelWireless = () => invoke("wireless_cancel");

export async function disconnectWireless() {
  await invoke("wireless_disconnect");
  wireless.set({ phase: "idle", step: null, ssid: null, error: null });
  await scan();
}

const isWireless = (path) => get(drives).some((tx) => tx.wireless && tx.takes.some((t) => t.files.some((f) => f.path === path)));
// Wireless files left to fetch once a USB import that ran first finishes.
let pendingWireless = [];

let started = false;
/** Poll for drives and listen for import events. Call once from the app. */
export async function start() {
  if (started) return;
  started = true;
  if (!get(dest)) {
    try {
      setDest(await invoke("recordings_default_dest"));
    } catch {
      /* leave empty: the panel asks for a folder */
    }
  }
  dest.subscribe(() => scan());
  setInterval(scan, 2500);
  await listen("recordings-progress", (e) => job.update((j) => ({ ...j, progress: e.payload })));
  await listen("recordings-done", (e) => {
    if (pendingWireless.length > 0) {
      const files = pendingWireless;
      pendingWireless = [];
      startWirelessImport(files).catch(() => job.set({ running: false, progress: null, outcome: e.payload }));
      return;
    }
    job.set({ running: false, progress: null, outcome: e.payload });
    scan();
  });
  await listen("wireless-step", (e) =>
    wireless.update((w) => ({ ...w, step: e.payload, ssid: e.payload.ssid ?? w.ssid })),
  );
  await listen("wireless-lost", () => {
    wireless.set({ phase: "idle", step: null, ssid: null, error: null });
    scan();
  });
}

export async function pickDest() {
  const picked = await invoke("recordings_pick_dest", { current: get(dest) || null });
  if (picked) setDest(picked);
}

async function startWirelessImport(files) {
  await invoke("wireless_import", { files, dest: get(dest) });
}

/** Import files from USB drives and/or the wireless transmitter (USB first). */
export async function importFiles(files) {
  const air = files.filter(isWireless);
  const usb = files.filter((f) => !isWireless(f));
  job.set({ running: true, progress: null, outcome: null });
  try {
    if (usb.length > 0) {
      pendingWireless = air;
      await invoke("recordings_import", { files: usb, dest: get(dest) });
    } else {
      await startWirelessImport(air);
    }
  } catch (e) {
    pendingWireless = [];
    job.set({ running: false, progress: null, outcome: null });
    throw e;
  }
}

export function cancelImport() {
  pendingWireless = [];
  invoke("recordings_cancel").catch(() => {});
  invoke("wireless_import_cancel").catch(() => {});
}
export const clearOutcome = () => job.update((j) => ({ ...j, outcome: null }));

export async function deleteFiles(files) {
  const result = await invoke("recordings_delete", { files, dest: get(dest) });
  await scan();
  return result;
}

export const openDest = () => invoke("recordings_open_dest", { dest: get(dest) });

export async function eject(mount) {
  if (mount.startsWith("wifi://")) return disconnectWireless();
  await invoke("recordings_eject", { mount });
  await scan();
}
