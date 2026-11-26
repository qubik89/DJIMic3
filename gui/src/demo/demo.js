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

// What the diagnostics window shows, in the shape `DeviceManager::diagnostics`
// writes it.
const DIAGNOSTICS = `DJI Mic Control diagnostic report
app 1.6.0 · macos aarch64 · 2026-11-26 10:12 UTC
supported receivers on the bus: 1 · opened: 1

== DJI USB devices ==
2ca3:4015 "DJI Mic 3 RX" by "DJI" · high speed · bus 1 address 3
  device class ef/02/01 (miscellaneous)
  interface 0: 01/01/00 (audio)
  interface 1: 01/02/00 (audio)
  interface 4: ff/f0/00 (vendor specific) "com.dji.mic"
  app: DJI Mic 3 (interface 4) · opened

== Devices the app opened ==
…7Q2K · DJI Mic 3 (dji-mic-3) · link open · last status 0.3s ago
  link open for 41.2s · 96 reads (11904 bytes) · 88 frames
  frames by cmd set/id: 00/01×1 2b/03×80 2b/06×7
  commands sent (latest 24):
    +1.1s connect packet → ok
      55 1a 04 b1 02 5a 01 00 40 00 33 ...
    +18.4s set noise-cancel=strong → ok
      55 13 04 03 02 5a 05 00 40 2b 02 ...
  first frames:
    55 36 04 1c 5a 02 3c 2f 00 2b 03 ...`;

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
      case "diagnostics":
        return DIAGNOSTICS;
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
