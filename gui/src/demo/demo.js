// Development only: runs the real UI against a simulated backend, so the
// website's screenshots can be taken in every language without hardware.
// The data mirrors what the Rust commands send: the setting descriptors are
// the Mic 3's own (mic3-settings.json, from
// `cargo run -p protocol --example demo_settings`), the rest follows the
// serde shapes of Snapshot, DeviceStatus and recordings::Transmitter.
//
//   demo.html?lang=es              language (en, es, fr, de, it, nl)
//   demo.html?lang=es&devices=0    no receiver plugged in (the start screen)
//   demo.html?lang=es&scene=live   both transmitters recording, TX2 muted
//
// window.demo.step(unit, step) pushes a wireless step to the wizard, e.g.
// demo.step("tx", { step: "confirm", pin: "4821" }).
// window.demo.advance(secs) moves the clock forward, so the recording timers
// show a take in progress (they count from when recording was first seen).
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { snapshot, drives, liveScene, DEST } from "./data.js";

const params = new URLSearchParams(location.search);
const lang = params.get("lang");
if (lang) localStorage.setItem("djimic.locale", lang);
const withDevice = params.get("devices") !== "0";
if (params.get("scene") === "live") liveScene();
let clockOffset = 0;
const realNow = Date.now.bind(Date);
Date.now = () => realNow() + clockOffset;
// Fresh state on every load: no remembered folder or file kind.
localStorage.removeItem("djimic.recordings.dest");
localStorage.removeItem("djimic.recordings.kind");

mockWindows("main");
mockIPC(
  (cmd, args) => {
    switch (cmd) {
      case "snapshot":
        return snapshot(withDevice);
      case "recordings_default_dest":
        return DEST;
      case "recordings_scan":
        return withDevice || params.get("drives") === "1" ? drives() : [];
      case "rx_wireless_devices":
        return [];
      // Pairing waits for the person to confirm on the unit: never resolve,
      // the steps come from demo.step().
      case "wireless_connect":
      case "rx_wireless_connect":
        return new Promise(() => {});
      default:
        if (!cmd.startsWith("plugin:")) console.debug("demo: unhandled", cmd, args);
        return null;
    }
  },
  { shouldMockEvents: true },
);

window.demo = {
  step: (unit, step) => emit(unit === "rx" ? "wireless-rx-step" : "wireless-step", step),
  advance: (secs) => (clockOffset += secs * 1000),
};

await import("../main.js");
