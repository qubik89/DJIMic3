// Sample data for the demo page (see demo.js): a DJI Mic 3 receiver with two
// transmitters, and the takes on both transmitters in the charging case.
import settings from "./mic3-settings.json";

const FW = "03.01.00.10";
export const DEST = "~/Music/DJI Mic";

// Battery is the raw 1–7 gauge: 1 is full, 7 is empty.
const tx = (level, battery, tone, gain, remaining) => ({
  level,
  serial: null,
  firmware: FW,
  product_name: "DJI Mic 3",
  voice_tone: tone,
  charging: false,
  battery,
  settings: { mute: "off", record: "off", "voice-tone": tone, "tx-gain": gain },
  rec_total: 430,
  rec_remaining: remaining,
});

const status = {
  model_id: "dji-mic-3",
  connected: true,
  nc_enabled: true,
  tx: [tx(64, 2, "rich", "0", 371), tx(41, 3, "standard", "p2", 388), null, null],
  rx: {
    serial: "9SEXNC5047D21R",
    firmware: FW,
    battery: 2,
    charging: false,
    product_name: "DJI Mic 3",
    hardware: null,
    environment: "indoors",
  },
  settings: {
    "noise-cancel": "basic",
    "noise-cancel-power": "on",
    "noise-cancel-button": "on",
    "low-cut": "on",
    stereo: "stereo",
    "rx-gain": "0",
    lossless: "on",
    "monitor-gain": "0",
    "tx-clip-control": "on",
    "loudness-balance": "off",
    "auto-off-15m": "on",
    "tx-auto-off-15m": "on",
    "camera-power": "off",
    "plug-free": "off",
    "mic-leds": "on",
    vibration: "on",
    float32: "on",
    "loop-recording": "off",
    "startup-recording": "off",
    "low-power-recording": "on",
    "rec-stop": "on",
  },
  protocol_version: 2,
  gain_dial: null,
};

// The "live" scene (the website's main screenshot): both transmitters
// recording, TX2 muted.
export function liveScene() {
  for (const t of status.tx.slice(0, 2)) t.settings.record = "on";
  status.tx[1].settings.mute = "on";
}

export function snapshot(withDevice) {
  if (!withDevice) {
    return { os: "macos", probe: { os: "macos", present: 0, accessible: 0, permission_issue: false }, devices: [], status: null, settings: [] };
  }
  return {
    os: "macos",
    probe: { os: "macos", present: 1, accessible: 1, permission_issue: false },
    devices: [
      {
        id: "usb-1",
        model_id: "dji-mic-3",
        model_name: "DJI Mic 3",
        pictogram_key: "mic-3",
        connected: true,
        rx_serial: status.rx.serial,
        tx: [{ battery: 2, charging: false }, { battery: 3, charging: false }, null, null],
        max_tx: 4,
      },
    ],
    status,
    settings,
  };
}

// ---- Recordings -------------------------------------------------------------

// Mono 48 kHz: 24-bit is 144 000 bytes a second, 32-bit float 192 000; the
// header (with its bext/iXML chunks) is 32 776 bytes.
const size = (secs, float) => 32776 + Math.round(secs * (float ? 192000 : 144000));

function take(mount, folder, slot, n, date, time, secs, float, imported) {
  const key = `TX0${slot}_MIC${String(n).padStart(3, "0")}_${date}_${time}`;
  const iso = `${date.slice(0, 4)}-${date.slice(4, 6)}-${date.slice(6)}T${time.slice(0, 2)}:${time.slice(2, 4)}:${time.slice(4)}`;
  const file = (suffix, kind) => ({
    name: `${key}_${suffix}.wav`,
    path: `${mount}/${folder}/${key}_${suffix}.wav`,
    kind,
    size: size(secs, float),
    imported,
  });
  return {
    key,
    slot,
    number: n,
    recorded_at: iso,
    info: {
      channels: 1,
      sample_rate: 48000,
      bits_per_sample: float ? 32 : 24,
      float,
      duration_secs: secs,
      originator: "MIC 3",
      description: `ver:${FW}`,
      origination: [iso.slice(0, 10), iso.slice(11)],
    },
    files: [file("orig", "original"), file("edit", "processed")],
  };
}

function transmitter(mount, folder, slot, rows) {
  return {
    id: folder,
    mount,
    slot,
    model: "MIC 3",
    firmware: FW,
    wireless: false,
    takes: rows.map((r) => take(mount, folder, slot, ...r)),
  };
}

export function drives() {
  return [
    transmitter("/Volumes/NO NAME", "TX_MIC001_20261103_101522", 1, [
      [9, "20261123", "183012", 2468, true, false],
      [8, "20261123", "171544", 1802, true, false],
      [7, "20261121", "112705", 3215, true, false],
      [6, "20261118", "094130", 1311, false, true],
      [5, "20261117", "121733", 846, false, true],
    ]),
    transmitter("/Volumes/NO NAME 1", "TX_MIC001_20261103_101809", 2, [
      [7, "20261123", "183014", 2466, true, false],
      [6, "20261123", "171546", 1799, true, false],
      [5, "20261121", "112708", 3209, true, false],
      [4, "20261118", "094133", 1308, false, true],
    ]),
  ];
}
