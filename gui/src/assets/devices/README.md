# Device pictograms

Transparent device artwork shown in the sidebar and status cards. Files are
matched by **filename stem**, built from a model's `pictogram_key` plus a part
suffix:

| Where shown                     | File (for the Mic Mini)  |
| ------------------------------- | ------------------------ |
| Sidebar entry, panel header, Receiver card | `mic-mini-rx.png` |
| Transmitter cards               | `mic-mini-tx.png`        |

So a model with `pictogram_key = "mic-mini"` looks for `mic-mini-rx.*` and
`mic-mini-tx.*` (`.png`, `.svg`, or `.webp`).

- Prefer transparent PNG/SVG; any aspect ratio works (images are contained, not
  cropped). Larger is better for crispness on high-DPI screens.
- If a file is missing, a neutral built-in microphone glyph is shown instead, so
  the app still works before artwork is supplied.

Adding a file here is picked up automatically — no code changes needed.

A connected transmitter can also override its own card artwork based on the
product name it reports, independent of the receiver's `pictogram_key` — see
`TX_PICTOGRAMS` in `DevicePanel.svelte`. `mic-mini-2-tx.png` is the DJI Mic
Mini 2's picture, used when a TX's `product_name` is `"DJI Mic Mini 2"`;
anything else (or not yet known) falls back to the base model's TX artwork.

`mic-3-rx.png` / `mic-3-tx.png` are DJI Mic 3 product photos with the
background removed. The Mic 3 never reports a transmitter product name, so
its TX cards use `mic-3-tx` once the transmitter's serial has arrived.

Other product photos (background removed): `mic-mini-2s-tx.png`,
`mic-2-tx.png`, `mobile-rx.png` (pictogram key `mobile`). The Mic Mini 2S
receiver uses the Mic Mini receiver's photo (`mic-mini-2s-rx.png`); they
share the same body. Transmitter cards pick artwork by the product name each
transmitter reports (`TX_PICTOGRAMS` in `DevicePanel.svelte`).
