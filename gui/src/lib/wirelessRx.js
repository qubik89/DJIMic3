// A receiver controlled over its own Wi-Fi instead of USB (desktop only).
// Once connected it shows up as a regular device in the snapshot; this
// module keeps the guided connection's state and which devices are wireless.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { writable } from "svelte/store";
import { region } from "./recordings.js";

/** phase "idle" | "connecting" | "connected" | "error", latest step, Wi-Fi name, error. */
export const rxWireless = writable({ phase: "idle", step: null, ssid: null, error: null });
/** Device ids of receivers connected over Wi-Fi. */
export const rxDevices = writable([]);

let started = false;
export async function start() {
  if (started) return;
  started = true;
  try {
    rxDevices.set(await invoke("rx_wireless_devices"));
  } catch {
    /* desktop only */
  }
  await listen("wireless-rx-step", (e) => rxWireless.update((w) => ({ ...w, step: e.payload, ssid: e.payload.ssid ?? w.ssid })));
  await listen("wireless-rx-lost", (e) => {
    rxDevices.update((list) => list.filter((id) => id !== e.payload));
    rxWireless.set({ phase: "idle", step: null, ssid: null, error: null });
  });
}

/** Pair with a receiver over Bluetooth and join its Wi-Fi. Resolves to the device id. */
export async function connectRx() {
  rxWireless.set({ phase: "connecting", step: { step: "scanning" }, ssid: null, error: null });
  try {
    const id = await invoke("rx_wireless_connect", { country: region() });
    rxDevices.update((list) => [...new Set([...list, id])]);
    rxWireless.update((w) => ({ ...w, phase: "connected", step: { step: "ready" } }));
    return id;
  } catch (e) {
    const err = typeof e === "object" && e ? e : { kind: "session", detail: String(e) };
    rxWireless.update((w) => (err.kind === "cancelled" ? { ...w, phase: "idle", step: null } : { ...w, phase: "error", error: err }));
    return null;
  }
}

export const cancelRx = () => invoke("rx_wireless_cancel");

export async function disconnectRx(id) {
  await invoke("rx_wireless_disconnect", { device: id });
  rxDevices.update((list) => list.filter((d) => d !== id));
  rxWireless.set({ phase: "idle", step: null, ssid: null, error: null });
}
