// Scene presets: several settings applied at once. Keys are setting ids;
// "voice-tone" is applied to every connected transmitter. Keys a model
// doesn't have are skipped.
export const SCENES = [
  {
    id: "interview",
    label: "Interview",
    desc: "Stereo split, gentle noise reduction, rich tone",
    set: { "noise-cancel-power": "on", "noise-cancel": "basic", "low-cut": "on", stereo: "stereo", "tx-clip-control": "on", "loudness-balance": "off", "voice-tone": "rich" },
  },
  {
    id: "outdoor",
    label: "Outdoor",
    desc: "Strong noise reduction, low cut, bright tone",
    set: { "noise-cancel-power": "on", "noise-cancel": "strong", "low-cut": "on", stereo: "mono", "tx-clip-control": "on", "loudness-balance": "on", "voice-tone": "bright" },
  },
  {
    id: "vlog",
    label: "Vlog",
    desc: "Mono mix, even loudness, standard tone",
    set: { "noise-cancel-power": "on", "noise-cancel": "basic", "low-cut": "on", stereo: "mono", "tx-clip-control": "off", "loudness-balance": "on", "voice-tone": "standard" },
  },
  {
    id: "podcast",
    label: "Podcast",
    desc: "Stereo, lossless, 32-bit float backup",
    set: { "noise-cancel-power": "off", "low-cut": "off", stereo: "stereo", lossless: "on", "tx-clip-control": "on", "loudness-balance": "off", "voice-tone": "rich", float32: "on" },
  },
];
