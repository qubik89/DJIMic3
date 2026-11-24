# DJI Mic Mini USB protocol

The DJI Mic Mini is controlled over USB Bulk transfers on **interface 6**
(vendor-specific). Two endpoints carry all traffic:

- **Bulk OUT `0x06`** — host → device (commands)
- **Bulk IN `0x86`** — device → host (acknowledgements and heartbeats)

USB identifiers: **vendor `0x2ca3`, product `0x4011`**.

Multi-byte integer fields are little-endian unless stated otherwise.

## Protocol versions

A firmware update changed the wire protocol: a different CRC-16 seed, a
target-addressed command shape, a restructured heartbeat, and several new
settings. The prior protocol is referred to as **v1** below and the new one
as **v2**; they otherwise share the same framing and the same 14-byte ACK. A
device's version is detected from its heartbeat stream (see
[Heartbeat marker](#heartbeat-marker-and-version-detection)) — there is no way
to ask for it directly.

Everything under "v1" and "v2" headings below is version-specific; everything
else applies to both.

---

## Packet framing

Every packet uses the DUML framing convention:

- Byte `0` is `0x55`, the start-of-frame marker.
- Byte `1` is the total packet length in bytes, including the header and the
  two-byte CRC at the end.
- Byte `2` is the protocol version, always `0x04`.
- Byte `3` is a CRC-8 (polynomial `0x31`, initial value `0xEE`, reflected
  in/out) computed over bytes `0`–`2`. Since those three bytes are otherwise
  fixed, this works out to a value that depends only on the packet's length
  (e.g. `0x03` for a 19-byte v1 command, `0xfc` for a 22-byte v2 one) — **it
  is not a routing/direction field**, despite v1 firmware only ever emitting
  two lengths (19 and 14) and thus only ever showing two values here. v2
  firmware's wider variety of packet lengths makes the actual pattern visible.
- The final two bytes are a CRC-16 (see below).

To read a stream, scan for `0x55` whose offset-2 byte is `0x04`, then take
`length` bytes.

---

## CRC-16

Packets end with a 2-byte CRC-16 built on the reflected `0x8408` polynomial
(same polynomial as CRC-16/IBM-SDLC / X-25). The two CRC bytes are chosen so
that checksumming the *entire* packet — body and CRC bytes together — always
lands on a fixed residue, for any content. This lets both directions validate
received packets and compute a checksum for outgoing ones without inspecting
a candidate CRC value directly.

| Version | Initial value | Residue  |
| ------- | -------------- | -------- |
| v1      | `0x0000`       | `0xbb01` |
| v2      | `0x3692`       | `0x0000` |

The polynomial and table are identical between versions; only the seed (and
therefore the residue it produces) changed.

---

## Command packets (host → device)

Sent to change a setting. The device replies with a [14-byte ACK](#ack-packets-device--host)
in both versions.

### v1

**Length: 19 bytes.** Example (Noise Cancel → Strong):

```
55 13 04 03 02 5a 01 00 40 5b 01 00 03 1d 00 01 01 91 fb
```

| Offset | Len | Value    | Description                                   |
| ------ | --- | -------- | --------------------------------------------- |
| 0      | 1   | `0x55`   | Start of frame                                |
| 1      | 1   | `0x13`   | Packet length (19)                            |
| 2      | 1   | `0x04`   | Protocol version                              |
| 3      | 1   | `0x03`   | Header CRC-8 for a 19-byte frame               |
| 4      | 1   | `0x02`   | Command set (audio subsystem)                 |
| 5      | 1   | `0x5a`   | Attribute flags                               |
| 6      | 2   | seq LE   | Sequence number, increments per command       |
| 8      | 1   | `0x40`   | ACK requested                                 |
| 9      | 1   | `0x5b`   | Constant                                      |
| 10     | 1   | `0x01`   | Constant                                      |
| 11     | 1   | `0x00`   | Constant                                      |
| 12     | 2   | cmd BE   | Command id, 16-bit **big-endian**             |
| 14     | 1   | `0x00`   | Constant                                      |
| 15     | 1   | `0x01`   | Payload length (1)                            |
| 16     | 1   | value    | Command value                                 |
| 17     | 2   | CRC      | See above                                     |

### v2

**Length: 22 bytes.** Same preamble as v1, but with an extra sub-type byte,
a 16-bit **target** address, and the command id written **little-endian**
instead of big-endian. Example (Noise Cancel → Basic, broadcast to both TX):

```
55 16 04 fc 02 5a 40 b6 40 5b 01 02 ff ff 00 00 37 00 01 00 dd e2
```

| Offset | Len | Value    | Description                                        |
| ------ | --- | -------- | --------------------------------------------------- |
| 0      | 1   | `0x55`   | Start of frame                                     |
| 1      | 1   | `0x16`   | Packet length (22)                                 |
| 2      | 1   | `0x04`   | Protocol version                                   |
| 3      | 1   | `0xfc`   | Header CRC-8 for a 22-byte frame                    |
| 4      | 1   | `0x02`   | Command set (audio subsystem)                       |
| 5      | 1   | `0x5a`   | Attribute flags                                    |
| 6      | 2   | seq LE   | Sequence number, increments per command             |
| 8      | 1   | `0x40`   | ACK requested                                      |
| 9      | 1   | `0x5b`   | Constant                                           |
| 10     | 1   | `0x01`   | Constant                                           |
| 11     | 1   | `0x02`   | Target-addressed command sub-type                   |
| 12     | 2   | target LE| `0x0000` = receiver, `0xffff` = broadcast to TX, or `0x0001`/`0x0002` for one specific TX unit (see Voice Tone) |
| 14     | 2   | `0x0000` | Constant                                           |
| 16     | 2   | cmd LE   | Command id, 16-bit **little-endian**                |
| 18     | 1   | `0x01`   | Payload length (1)                                  |
| 19     | 1   | value    | Command value                                      |
| 20     | 2   | CRC      | See above                                          |

### Commands

Every setting below works in both versions except the three marked "new" —
those don't exist on v1 firmware at all.

| Setting                              | v1 cmd     | v2 cmd       | Target      | Off/On values                    |
| ------------------------------------- | ---------- | ------------ | ----------- | --------------------------------- |
| Noise cancel mode (Basic/Strong)      | `0x031d`   | `0x0037`     | broadcast   | `0x00` Basic, `0x01` Strong       |
| Noise cancel enabled (**new**)        | —          | `0x0038`     | broadcast   | `0x00` off, `0x01` on             |
| NC toggle via TX button (**new**)     | —          | `0x000f`     | broadcast   | `0x00` off, `0x01` on             |
| Low cut                               | `0x0303`   | `0x0003`     | broadcast   | `0x00` off, `0x01` on             |
| Stereo tracking                       | `0x0008`   | `0x0008`     | receiver    | `0x00` off/mono, `0x02` on        |
| Safety track                          | `0x0021`   | `0x0021`     | receiver    | `0x00` off, `0x01` on             |
| Clip limiter                          | `0x001e`   | `0x001e`     | receiver    | `0x00` off, `0x01` on             |
| Auto off after 15 min (receiver)      | `0x0010`   | `0x0010`     | receiver    | `0x00` off, `0x01` on             |
| Auto off after 15 min (TX, **new**)   | —          | `0x0010`     | broadcast   | `0x00` off, `0x01` on             |
| Turn on with camera                   | `0x0020`   | `0x0020`     | receiver    | `0x00` off, `0x01` on             |
| Plug-free external speaker            | `0x0023`   | `0x0023`     | receiver    | `0x00` off, `0x01` on             |
| Mic LEDs                              | `0x030a`   | `0x000a`     | broadcast   | `0x00` on, `0x02` off (inverted)  |
| Voice Tone (**new**, Mic Mini 2 only)  | —          | `0x0029`     | one TX unit | `0x00` Standard, `0x01` Rich, `0x02` Bright |

Notes:

- Stereo tracking uses `0x02` for On (not `0x01`) — unchanged between versions.
- Mic LED values are inverted: `0x00` = On, `0x02` = Off — unchanged between
  versions, but v2 firmware issued it a new command id anyway.
- Toggling the plug-free external speaker reboots the receiver; the new state
  appears in heartbeats after the reboot completes.
- "NC toggle via TX button" does not itself enable/disable noise
  cancellation — it enables/disables whether a quick single press of the
  transmitter's power button can. Enabling/disabling NC itself is the
  separate "Noise cancel enabled" command above.
- Auto off (RX) and auto off (TX) share one command id, distinguished purely
  by target address — v1 firmware apparently only ever exposed the
  receiver's timer.
- Whether any setting besides noise cancel mode has a *different* command id
  between versions beyond what's listed here is unconfirmed — everything else
  reused its v1 id verbatim.
- Voice Tone is the only setting that targets one specific transmitter
  (`0x0001` or `0x0002`) rather than broadcasting to both — every other TX
  setting mirrors identically across whichever units are connected. It's
  also the only setting confirmed to exist on one hardware generation (DJI
  Mic Mini 2) but not another (the original DJI Mic Mini); every other
  setting has been exercised on both.

---

## ACK packets (device → host)

Returned for every command, in both versions, in the same 14-byte shape:

```
55 0e 04 66 5a 02 01 00 80 5b 01 00 c0 e8
```

| Offset | Len | Value  | Description                       |
| ------ | --- | ------ | ---------------------------------- |
| 0      | 1   | `0x55` | Start of frame                    |
| 1      | 1   | `0x0e` | Packet length (14)                |
| 2      | 1   | `0x04` | Protocol version                  |
| 3      | 1   | `0x66` | Header CRC-8 for a 14-byte frame  |
| 4      | 1   | `0x5a` | Attribute flags                   |
| 5      | 1   | `0x02` | Command set (audio subsystem)     |
| 6      | 2   | seq LE | Sequence echoed from the command  |
| 8      | 1   | `0x80` | ACK flag                          |
| 9      | 1   | `0x5b` | Constant                          |
| 10     | 1   | `0x01` | Constant                          |
| 11     | 1   | `0x00` | Constant                          |
| 12     | 2   | CRC    | See above                         |

---

## Heartbeat marker and version detection

Every heartbeat/status push shares this header shape at offset 8–11
regardless of version or sub-type:

| Offset | Len | Value      | Description                                    |
| ------ | --- | ---------- | ----------------------------------------------- |
| 8      | 1   | `0x00`     | No ACK required                                 |
| 9      | 2   | `5b 03`    | Constant                                        |
| 11     | 1   | `0x00`/`0x03` | **Version marker**: `0x00` = v1, `0x03` = v2 |

This is the only reliable way to tell which version a connected device speaks.

---

## v1 heartbeat

Streamed continuously (~10 Hz) on the Bulk IN endpoint, carrying current
state and per-transmitter data in one push. Packet size depends on how many
transmitters are powered on:

| Variant | Magic         | Length   | TX slots            |
| ------- | ------------- | -------- | ------------------- |
| 0-TX    | `55 38 04 e1` | 56 bytes | both absent         |
| 1-TX    | `55 46 04 8b` | 70 bytes | one present, one stub |
| 2-TX    | `55 54 04 f6` | 84 bytes | both present        |

All variants share a 14-byte header, two TX slots, a 22-byte RX entry, and a CRC.

### Header (14 bytes)

| Offset | Len | Value       | Description                                   |
| ------ | --- | ----------- | --------------------------------------------- |
| 0      | 1   | `0x55`      | Start of frame                                |
| 1      | 1   | length      | `0x38` / `0x46` / `0x54`                       |
| 2      | 1   | `0x04`      | Protocol version                              |
| 3      | 1   | type        | `0xe1` / `0x8b` / `0xf6` (header CRC-8, incidentally encodes TX count via length) |
| 4      | 1   | `0x5a`      | Attribute flags                               |
| 5      | 1   | `0x02`      | Command set (audio subsystem)                 |
| 6      | 2   | seq LE      | Sequence number                               |
| 8      | 1   | `0x00`      | No ACK required                               |
| 9      | 3   | `5b 03 00`  | Heartbeat marker (version = v1)               |
| 12     | 1   | —           | Device sub-state                              |
| 13     | 1   | `0x00`      | Constant                                      |

### TX slots

Starting at offset 14, each of the two slots is either a **present entry**
(23 bytes) or an **absent stub** (9 bytes). Byte `+1` of a slot distinguishes
them: `& 0x20` set → present, `& 0x40` set → absent. Slot 0 is TX1, slot 1 is TX2.

**Present TX entry (23 bytes):**

| Slot offset | Len | Description                                                    |
| ----------- | --- | ---------------------------------------------------------------- |
| +0          | 1   | State byte (see below)                                         |
| +1          | 1   | TX flags (see below)                                           |
| +2          | 1   | Audio input level                                              |
| +3          | 1   | LED flags — bit `0x80` set = LEDs off                          |
| +4          | 4   | Firmware version (one byte per component, decimal)             |
| +8          | 1   | Constant `0x0e`                                                |
| +9          | 14  | Transmitter serial number (ASCII)                              |

**Absent TX stub (9 bytes):** `+0` state byte, `+1` TX flags, `+2`..`+8` zero.
The state and flags bytes are valid even in a stub.

**State byte (`+0`):**

| Mask   | Meaning                                  |
| ------ | ----------------------------------------- |
| `0x20` | Strong noise-cancel mode (clear = Basic)  |
| `0x04` | Low cut active                           |

**TX flags byte (`+1`):**

| Mask   | Meaning                                              |
| ------ | ------------------------------------------------------ |
| `0x20` | Present TX marker                                     |
| `0x40` | Absent stub marker                                    |
| `0x01` | Noise cancel enabled (toggled by the TX's button)      |

### RX entry (22 bytes)

Follows the two TX slots (absolute offset 32 / 46 / 60 for 0 / 1 / 2-TX).

| RX offset | Len | Description                                        |
| --------- | --- | ---------------------------------------------------- |
| +0        | 1   | RX flags byte 0 (see below)                        |
| +1        | 1   | RX flags byte 1 (see below)                        |
| +2        | 4   | Firmware version (one byte per component, decimal) |
| +6        | 1   | Constant `0x0e`                                    |
| +7        | 14  | Receiver serial number (ASCII)                     |
| +21       | 1   | Plug-free external speaker: `0x01` on, `0x00` off  |

**RX flags byte 0:**

| Mask   | Meaning                    |
| ------ | ---------------------------- |
| `0x01` | Turn on with camera active |
| `0x02` | Auto off after 15 min active |
| `0x08` | Stereo tracking active     |

**RX flags byte 1:**

| Mask   | Meaning              |
| ------ | ---------------------- |
| `0x80` | Safety track active  |
| `0x20` | Clip limiter active  |

### Example: 2-TX heartbeat (84 bytes, serials are placeholder values)

```
55 54 04 f6 5a 02 0e 79 00 5b 03 00 47 00        header
b0 25 38 00 01 01 00 38 0e 42 42 42 42 42 42 42 42 42 42 42 42 42 42   TX1
b0 25 38 00 01 01 00 38 0e 43 43 43 43 43 43 43 43 43 43 43 43 43 43   TX2
32 20 01 01 00 38 0e 41 41 41 41 41 41 41 41 41 41 41 41 41 41 00   RX
<crc>
```

Decoded:

- TX1 — state `0xb0` (Strong NC, low cut off), level `0x38`, serial `BBBBBBBBBBBBBB`
  (placeholder), firmware `01.01.00.56`.
- TX2 — serial `CCCCCCCCCCCCCC` (placeholder).
- RX — flags `0x32`/`0x20` (auto-off on, clip on, others off), serial
  `AAAAAAAAAAAAAA` (placeholder), plug-free off.

---

## v2 status push

v2 firmware splits what v1 firmware pushed as one heartbeat into **three
separate periodic pushes** sharing the marker described above
([offset 11 = `0x03`](#heartbeat-marker-and-version-detection)):

1. **Status push** (this section) — compact flags plus a fixed-size slot per
   connected transmitter. Streamed continuously (~10 Hz), like the v1
   heartbeat.
2. **Identity push** — receiver/transmitter serial and product name, TLV-
   encoded. Sent once at connect (and again after a reconnect, e.g. the
   reboot a plug-free toggle causes), not on every tick.
3. **Audio-level push** — live per-transmitter input level, sent at roughly
   the same ~10 Hz rate as the status push but as its own message.

Frame *length alone* doesn't identify which of the three a given push is —
the identity push's length varies with how many units it lists, and can
coincide with a length the status push also uses — so decoding tries the
status push's fixed shape first and falls back to parsing the other two as
generic records (below); an unrecognised frame simply matches neither and is
ignored.

Since identity and level arrive on their own pushes rather than inside the
status push, a freshly connected device reports serial/firmware/level as
unknown for a moment until those pushes arrive; from then on, each new status
push must carry forward whatever was already learned rather than resetting it
to unknown on every tick (only dropping it if that transmitter disconnects).

### Status push shape

A fixed 52-byte header, then one 32-byte slot per **connected** transmitter
(0, 1, or 2 — there is no "absent stub" the way v1 firmware has; a
disconnected transmitter simply has no slot at all), then a 2-byte CRC:

| Variant | Length   | TX slots |
| ------- | -------- | -------- |
| 0-TX    | 54 bytes | none     |
| 1-TX    | 86 bytes | one      |
| 2-TX    | 118 bytes| two      |

Header offsets that matter (the rest is currently-unidentified constants):

| Offset | Len | Description                                                          |
| ------ | --- | ----------------------------------------------------------------------- |
| 0–11   | 12  | Standard preamble; see [framing](#packet-framing) and [marker](#heartbeat-marker-and-version-detection) |
| 12     | 1   | Capability byte: `0x26`/`0x46`/`0x66` for 0/1/2 TX connected (redundant with frame length) |
| 20     | 1   | Flags — see below                                                      |
| 21     | 1   | Flags — see below                                                      |
| 22     | 1   | Receiver's physical gain dial, signed, in dB — see below              |
| 44     | 1   | Per-TX connected bitmask — see below                                  |
| 48     | 1   | Flags — see below                                                      |
| 52     | 32×N| One slot per connected TX (see below)                                  |

**Offset 20:**

| Mask   | Meaning                              |
| ------ | --------------------------------------- |
| `0x08` | Strong noise-cancel mode (clear = Basic) |
| `0x20` | Noise cancel enabled                    |
| `0x80` | Turn on with camera active              |

**Offset 21:**

| Mask   | Meaning                    |
| ------ | ---------------------------- |
| `0x01` | Auto off (receiver) active |
| `0x04` | Stereo tracking active     |

**Offset 22:** the receiver's physical gain dial, a signed byte read directly
as dB — no scaling. The dial steps in fixed 6 dB increments across five
positions: `12`, `6`, `0`, `-6`, `-12`.

**Offset 44:** per-TX connected bitmask — `0x01` = TX1, `0x02` = TX2 (physical
unit numbering, not slot position). This is the authoritative signal for
which transmitter(s) are connected: it's set the instant a TX connects, up
to one status-push tick *before* that TX's 32-byte slot appears in the slot
list below, never the reverse — a decoder relying on slot presence alone
lags this bitmask by that one tick.

**Offset 48:**

| Mask   | Meaning                  |
| ------ | --------------------------- |
| `0x02` | Plug-free speaker active  |
| `0x10` | Clip limiter active      |
| `0x40` | Safety track active      |

**TX slot (32 bytes, at absolute offset `52 + 32×i`):**

The slot turns out to be a 6-byte record header — the same
`[tag:1][unit index:1][reserved:3][length:1]` shape used by the identity/
audio-level records below — followed by 26 bytes of data, of which only 4
are understood. Low Cut, Mic LEDs, TX Auto Off, and the NC-button permission
have **no header-level fallback** — they're only knowable when at least one
TX is connected, since they're purely per-transmitter settings. Everything
in this slot mirrors identically across whichever TX are present **except**
Voice Tone (`+9` bits `0x40`/`0x80`), the charging indicator (`+7` bit
`0x02`), and battery level (`+7` bits `0x1c`) — all independent
per transmitter, since each TX has its own tone setting, its own physical
dock/charge state, and its own battery level. See the Voice Tone note in
Commands above.

| Slot offset | Len | Description                                                        |
| ----------- | --- | --------------------------------------------------------------------- |
| +0          | 1   | Constant `0x02` (record tag)                                          |
| +1          | 1   | **Physical TX unit: `0x01` = TX1, `0x02` = TX2** — independent of slot position |
| +2          | 3   | Constant zero (reserved, matching the record header shape)            |
| +5          | 1   | Constant `0x1a` (26) — length of the data that follows, i.e. the rest of the slot |
| +6          | 1   | Flags: `0x02` LEDs off (inverted), `0x10` TX auto-off active, `0x20` strong NC (mirror), `0x80` NC-button-toggle permission active |
| +7          | 1   | Flags: `0x01` NC enabled (mirror of offset-20 bit `0x20`), `0x02` this TX is docked/charging (per-TX, not mirrored — reverts within a tick or two of undocking), `0x1c` battery level as a 3-bit gauge (per-TX — see below); constant bit `0x20` also set, purpose unknown |
| +8          | 1   | Flags: `0x04` clip limiter active (mirror of offset-48 bit `0x10`)    |
| +9          | 1   | Flags: `0x20` low cut active, `0x40` Voice Tone = Rich, `0x80` Voice Tone = Bright (neither set = Standard; Mic Mini 2 only — see Commands); a constant bit `0x01` also set, purpose unknown |
| +10         | 1   | Constant zero                                                          |
| +11         | 1   | Constant `0x78`, purpose unknown                                      |
| +12–+31     | 20  | Constant zero                                                         |

Since offset `+1` gives the physical TX directly, a decoder can place each
slot's data by real unit number and never needs to guess which physical
transmitter a lone connected slot belongs to.

**Battery level (`+7` bits `0x1c`, i.e. `0x04`/`0x08`/`0x10` together):**
independent per transmitter, and independent of the charging bit — both can
be set at once. These three bits form a single 3-bit gauge rather than
independent flags: read as `(byte >> 2) & 0x07`, higher means more
depleted. A value of `7` (all three bits set) is the terminal reading
immediately before the transmitter shuts off from an empty battery; `1`
(just `0x04`) corresponds to a full charge. The official app's low-battery
warning icon corresponds to a reading of `6` (`0x08` and `0x10` set, `0x04`
clear).

A reading of `0` (no bits set) has not been demonstrated to occur during
normal charge/discharge and is believed unreachable — the practical range
is `1`–`7`. The gauge does not update on every possible value during a fast
charge from empty: at least one intermediate value has been observed
skipped when charging resumes quickly after a full discharge, so a decoder
should not assume every value in range is guaranteed to appear.

### Identity and audio-level pushes: shared record format

Both push types are a sequence of records starting at offset 14, preceded by
a 16-bit little-endian length field at offset 12 giving the byte length of
everything from offset 14 up to (not including) the trailing CRC — i.e.
`frame length − 14 − 2`, redundant with the frame's own length:

```
[tag: 1][unit index: 1][reserved: 3][length: 1][data: length bytes]
```

repeated until the frame ends. Known tags:

| Tag    | Push     | Data                                                              | Unit index                          |
| ------ | -------- | ------------------------------------------------------------------ | -------------------------------------- |
| `0x01` | Identity | 4-byte firmware version (component order reversed — see below), then a 14-byte ASCII serial | `0` = receiver, `1`/`2` = TX slot 0/1 (0-based) |
| `0x04` | Identity | 6-character hex-like id (not modelled)                              | same as `0x01`                         |
| `0x06` | Identity | ASCII product name, e.g. `DJI Mic Mini` or `DJI Mic Mini 2` — variable length, matching the record's own length byte | same as `0x01`; only TX (`1`/`2`) is modelled |
| `0x05` | Level    | 1-byte live audio input level                                       | `1`/`2` = TX slot 0/1 (**1-based**, unlike the identity push) |

The `0x01` record's firmware bytes read `[b0, b1, b2, b3]` on the wire but
decode to version `b3.b2.b1.b0` (each byte is a decimal component, matching
the v1 firmware version encoding) — e.g. wire bytes `00 11 03 02` decode
to `02.03.17.00`.

The identity push is sent once at connect (and again after a reconnect, e.g.
the reboot a plug-free toggle causes) and periodically thereafter, listing
whichever units are currently connected: 70 bytes for the receiver alone, 124
bytes for the receiver plus one transmitter, **178 bytes for the receiver
plus both transmitters** — simply the same per-unit record block repeated
once per unit, each with its own index. 178 bytes is the largest
frame in either version: **`packet::take_frame`'s `MAX_FRAME` sanity limit
must stay above this**, since it was originally sized around v1's 84-byte
heartbeat and silently discarded this frame as an "implausible length" before
being raised. The level push is sent continuously (~10 Hz, like the status
push) and is a fixed 30 bytes for two transmitters.

Example identity push for a receiver-only connection (70 bytes, record
headers bolded via spacing; serial and hex id are placeholder values):

```
55 46 04 8b 5a 02 00 00 00 5b 03 03  36 00                              preamble
01 00 00 00 00  12  00 11 03 02 <serial: 14 bytes, "DDDDDDDDDDDDDD">    tag 0x01, unit 0, firmware 00 11 03 02
04 00 00 00  06  <hex id: 6 bytes>                                       tag 0x04, unit 0
06 00 00 00  0c  <product name: 12 bytes>                                tag 0x06, unit 0
<crc>
```

Firmware `00 11 03 02` decodes to `02.03.17.00`.

### Example: 2-TX status push (118 bytes)

```
55 76 04 a6 5a 02 00 00 00 5b 03 03 66 00 03 00 00 00 20 28 31 0c   header
00×22                                                                  (unidentified)
02 01 00 00 00 1a b8 25 04 01 00 78 00×20                             TX1
02 02 00 00 00 1a b8 25 04 01 00 78 00×20                             TX2
<crc>
```

Decoded: offset 20 = `0x28` (strong NC on, NC power on), offset 21 = `0x31`
(auto-off on, stereo off), offset 48 = `0x1c` (plug-free off, clip on, safety
off). Each TX slot opens `02 01`/`02 02` — record tag `0x02` plus the
physical unit (TX1/TX2) — followed by `00 00 00 1a` (reserved, then length
26). TX slot flags (+6) = `0xb8` (LEDs on, TX auto-off on, NC-button
permission on), low-cut byte (+9) = `0x01` (low cut off).

---

## DJI Mic 3

Observed on a Mic 3 receiver (firmware `03.01.00.10`), USB identifiers
**vendor `0x2ca3`, product `0x4015`** (`0x4115` while in four-channel mode —
switching to or from that mode makes the receiver re-enumerate).

The receiver speaks the v2 dialect described above, unchanged in framing,
CRC, command shape and push layout, but on a different interface:

- **Interface 4** (`com.dji.mic`, class `ff/f0/01`), **alternate setting 1**.
  Alternate setting 0 has no endpoints.
- **Bulk OUT `0x01`**, **Bulk IN `0x81`**.

No iAP/External Accessory handshake is needed: status pushes start as soon
as alternate setting 1 is selected.

Differences from the Mic Mini:

- **ACK result code.** Offset 11 of the 14-byte ACK is `0x00` when the
  command was applied and `0xff` when the receiver rejected it.
- **Dropped first command.** The first command written after opening the
  interface is sometimes neither applied nor acknowledged; re-sending it
  works.
- **Keep-alive push.** A 14-byte frame `… 00 5b 04 06 <crc>` arrives about
  twice a second. It needs no reply.
- **Status push rate** is about 1 Hz with no transmitter connected.

Receiver commands (target `0x0000`), confirmed by write and status readback:

| Setting                  | Cmd      | Values                                   | Status readback            |
| ------------------------ | -------- | ---------------------------------------- | -------------------------- |
| Audio channels           | `0x0008` | `0x00` mono, `0x02` stereo, `0x04` 4-ch  | offset 21: `0x04` / `0x08` |
| Receiver gain            | `0x0039` | signed dB: −12, −6, 0, +6, +12           | offset 22, signed          |
| Monitoring gain          | `0x0026` | signed dB, −12…+12 in 1 dB steps         | offset 27, signed          |
| Auto off (receiver)      | `0x0010` | `0x00` off, `0x01` on                    | offset 21 bit `0x01`       |
| Turn on with camera      | `0x0020` | `0x00` off, `0x01` on                    | offset 20 bit `0x80`       |

Rejected (`0xff`) with no transmitter connected: Safety track (`0x0021`) and
Clip limiter (`0x001e`).

Offset 21 also appears to carry the receiver's own battery: bit `0x10`
charging, bits `0xe0` a 3-bit gauge like the transmitter's (unconfirmed).

Rejected (`0xff`) even with a transmitter connected: Safety track
(`0x0021`), Clip limiter (`0x001e`), and per-transmitter `0x003e` / `0x003d`.

Transmitter commands, confirmed by write and status readback with two
transmitters connected. The broadcast ones in the Commands table above
(`0x0037`, `0x0038`, `0x000f`, `0x0003`, `0x000a`, `0x0010`) behave exactly
as on the Mic Mini. Slot offsets are relative to the 32-byte TX slot.

"Mirrored" settings are applied to every transmitter even when the command
is addressed to one (`0x0001`), so they're best sent as a broadcast.

| Setting                   | Cmd      | Scope     | Values                          | Slot readback        |
| ------------------------- | -------- | --------- | ------------------------------- | -------------------- |
| Clipping control          | `0x0024` | mirrored  | `0x00` off, `0x01` on           | `+8` bit `0x04`      |
| Loudness balance          | `0x002c` | mirrored  | `0x01` off, `0x02` on           | `+11` bit `0x80`     |
| Vibration                 | `0x0004` | mirrored  | `0x00` off, `0x01` on           | `+9` bit `0x01`      |
| 32-bit float recording    | `0x000c` | mirrored  | `0x00` off, `0x01` on           | `+9` bit `0x08`      |
| Loop recording            | `0x002a` | mirrored  | `0x00` off, `0x01` on           | `+11` bit `0x08`     |
| Low-battery auto record   | `0x002b` | mirrored  | `0x00` off, `0x01` on           | `+11` bit `0x10`     |
| Record on power-on        | `0x002e` | mirrored  | `0x00` off, `0x01` on           | `+8` bit `0x80`      |
| Audio recording via button | `0x000b` | mirrored | `0x00` off, `0x01` on           | `+11` bit `0x20`     |
| Voice tone                | `0x0029` | per TX    | `0x00`/`0x01`/`0x02`            | `+9` bits `0x40`/`0x80` |
| Transmitter gain          | `0x0039` | per TX    | signed dB, −12…+12              | `+13`, signed        |
| Internal recording        | `0x0002` | per TX    | `0x00` stop, `0x01` start       | `+9` bit `0x10`      |

Clock sync: a 29-byte command to the receiver, command `0x0033`, payload
length `0x08`, payload `09 YY MM DD hh mm ss 00` (year mod 100, local time).
ACKed with `0x00`; sent once on connect so internal recordings get correct
timestamps.

Receiver radio settings, header offset 20: bit `0x01` set = Environment
"Outdoors" (clear = "Indoors"), bit `0x04` = Lossless Audio. Lossless is
written with command `0x0027` (receiver target, `0x00`/`0x01`); no write
command for Environment is known, so it's read-only here. Mute is command
`0x0009` per transmitter, read back at slot `+6` bit `0x04`. Both commands,
and the environment bits, match
[SanMurakami/mic3-bridge](https://github.com/SanMurakami/mic3-bridge).

Receiver battery: header offset 21 bits `0xe0` are a 1 (full) … 7 gauge and
bit `0x10` is set while charging (seen as `0x51` = gauge 2, charging, while
powered over USB).

Rejected (`0xff`): channel mode value `0x01` (no safety-track channel mode),
`0x003e` and `0x003d` to any target.

**Do not send `0x0007` to a transmitter.** dji-mic-mo uses it (value `0x01`)
to format the transmitter's internal storage. Value `0x00` was sent once by
mistake while mapping this protocol: it was ACKed, storage capacity was
unchanged, and slot `+9` bit `0x04` has been set on that transmitter since.

Slot `+14` and `+16` are the transmitter's total and remaining internal
recording time, 16-bit little-endian, in tenths of an hour. The slot's unit
byte (`+1`) is a bitmask on this receiver (`1`, `2`, `4`, `8` for TX1–TX4).
Header offset 20 bit `0x40` is set while any transmitter is recording.

Command ids and offsets were cross-checked against
[usokawa/dji-mic-mo](https://github.com/usokawa/dji-mic-mo).

Safety Track does not exist on the Mic 3 (DJI dropped it; the manual lists
only Mono, Stereo and Quadraphonic). `0x0021` is rejected; channel-mode
values `0x03` and `0x06` are ACKed but change nothing (`0x03` makes the
receiver re-enumerate and come back in mono).

### Identifying the receiver

Two read-only requests (documented for the Mobile RX by
[piedol/lavdeck](https://github.com/piedol/lavdeck)) also work on the Mic 3:

- **Get version**, command set `0x00`, id `0x01`, empty payload. The Mic 3
  receiver answers with the hardware name **`DM303_RX`** (ASCII, NUL-padded)
  followed by its firmware version (reversed bytes, `0a 00 01 03` →
  `03.01.00.10`). The Mobile RX reports `DJI MIC MINI RX`. This is the
  reliable way to tell receivers apart: the Mic 3 and the Mic Mini 2S
  receivers share USB ids `2ca3:4015`/`4115`, and the Mic Mini and Mobile
  receivers share `2ca3:4011`.
- **Get capability**, command set `0x5B`, id `0x12`, payload
  `index (u32 LE) | capability id (u16 LE, 0 = all)`. On the Mic 3 it only
  returns three one-byte flags per unit (ids `0x0e`, `0x0f`, `0x10`; receiver
  `0/1/0`, each transmitter `1/1/1`), not a usable feature list.

The Mic 3 receiver never sends a product-name record (`0x06`) for itself; its
transmitters don't either, so they show as "TX1"/"TX2".

### Transmitter storage and recordings

Internal recordings never travel over the receiver link. The Mic 3
transmitter has no USB port of its own: in the charging case with the case
plugged in over USB-C (or on the magnetic cable), each transmitter enumerates
as its own USB mass-storage device behind the case's hub:

| Field | Value |
| --- | --- |
| USB id | `be57:0104` (vendor string `bestechnic`, product `Wireles Mic Tx`, sic) |
| Serial | `20151006.1` on every transmitter, so useless for telling them apart |
| Interface | class 8 (mass storage), subclass 6 (SCSI), protocol `0x50` (bulk-only) |
| Medium | "Mic Tx", 30.1 GB, FAT32 with no partition table, volume label `NO NAME` |

A receiver left in the case shows up behind the same hub with its usual id
(`2ca3:4015`); it has no storage interface.

Layout (firmware 02.00.06.01 and 03.01.00.10):

```
TX_MIC002_20261117_121733/                  one folder per transmitter
  TX01_MIC003_20261123_232008_orig.wav      original track
  TX01_MIC003_20261123_232008_edit.wav      processed copy (dual-file recording)
```

- `TX<nn>` is the receiver channel the transmitter was paired to, `MIC<nnn>`
  a running take number, then the local start date and time (correct thanks to
  the clock sync above). `_edit` files appear only with dual-file recording on;
  they have the same size as the `_orig` file and differ in the audio.
- The folder name is the transmitter's first take; it doesn't change when
  more takes are added, so the app uses it as the transmitter's id.
- Files are Broadcast WAV: `fmt ` (PCM, mono, 48 kHz, 24-bit; IEEE float with
  32-bit float recording), a 602-byte `bext` (description `ver:<firmware>`,
  originator `MIC 3`, origination date and time, time reference 0, no UMID or
  loudness), an `iXML` chunk at 29.97 fps NDF repeating the `bext` fields,
  an empty `cue ` chunk and a `PAD ` chunk that puts the audio at byte 32776.

The app's import (crate `recordings`) recognises a transmitter by that folder
pattern rather than the label or USB id, so it works the same on every OS.

### Bluetooth LE (receiver)

The Mic 3 receiver also speaks this protocol over Bluetooth LE — the link
DJI Mimo uses. Seen with firmware 03.01.00.10:

- Advertising name `DJI Mic 3 RX-<6 hex digits>`, connectable, service UUID
  `FFF0`, manufacturer data `aa08 b300 0088298563d7b1` (`aa08` + model
  `b3 00` + device bytes, the same shape as DJI's Osmo cameras).
- One primary service `FFF0` with three characteristics: `FFF3` and `FFF4`
  (read, write, notify, indicate) and `FFF5` (read, write without response,
  notify, indicate). Negotiated maximum write: 512 bytes with response, 248
  without.
- Device → app frames arrive as notifications on `FFF4`; app → device frames
  are written to `FFF5` without response. Framing, CRCs and addresses are
  exactly the USB ones (app `0x02`, receiver `0x5a`).
- No pairing step was needed: right after subscribing, the receiver sends
  `cmd set 0x5b, id 0x04` frames (payload `06`) about twice a second, and a
  **Get version** written to `FFF5` was answered on `FFF4` with `DM303_RX`
  and the firmware version, as over USB.

### Bluetooth LE (transmitter)

A Mic 3 transmitter advertises only for a short while after its link button
is held for two seconds (out of the case, powered on); linked to a receiver
and left alone, neither unit advertises.

- Advertising name `DJI Mic 3 TX-<6 hex digits>`, service `FFF0`,
  manufacturer data `aa08 b200 008829858065ee` (model `b2 00`; the receiver
  is `b3 00`), service data `FDEE` = `010107040411f81232534543ff001502514c`.
  It also appears to Bluetooth Classic inquiry with class of device
  `0x240404` (audio, wearable headset): the "Bluetooth mic" mode for phones.
- GATT: the same `FFF0` service as the receiver (`FFF3`/`FFF4`/`FFF5`), the
  standard LE Audio Volume Control service `1844` (`2B7D` state, `2B7E`
  control point, `2B7F` flags), and a vendor service
  `15F1E600-A277-43FC-A484-DD39EF8A9100` with `…E601` (read, notify,
  indicate) and `…E602` (write, write without response).
- Its DUML address is **`0x3a`**. Without pairing it pushes, about twice a
  second, `cmd set 0x5b, id 0x03` frames with the same record format as the
  receiver's USB pushes (a `0x20`-byte status record and a `0x24`-byte
  identity record carrying firmware `0a 00 01 03`, the serial number and the
  name suffix), plus `0x5b`/`0x04` frames with payload `05`.
- **Get version** addressed to `0x3a` answers `DM303_TX`, firmware
  03.01.00.10. The same request addressed to `0x5a` gets no answer.
- A bare DUML **File List** request (`cmd set 0x00, id 0x20`, no payload)
  gets no answer over BLE: files don't travel over Bluetooth.

### Wireless file download (DJI Mimo)

Captured from DJI Mimo 2.12.12 on iOS (PacketLogger) and reproduced from a
Mac. Mimo uses BLE only to pair and to get Wi-Fi credentials; the files
come over the unit's own Wi-Fi access point. Receiver and transmitters each
have one (`DJI Mic 3 RX-63d7b0`, `DJI Mic 3 TX-817dd8`). Sequence, all on
`FFF0` (app `0x02` → unit `0x3a`/`0x5a`):

1. Subscribe to notifications on `FFF4` only (Mimo writes `01 00` to its
   CCCD and nothing else).
2. Every ~2.5 s: `00/2b` to address `0xf0`, payload `04 00` (app heartbeat).
3. `00/32` payload `11` → serial number and date/time
   (`00 01 | year u16 | month day hour min sec | len | serial`); Mimo writes
   it back with `12 01 …` to set the clock when the unit answers zeros.
4. **Pair**: `07/45`, payload `20 | 32-char ASCII app id | 04 | 4-digit
   ASCII PIN (random each time) | 01 01`. Answer `00 02` = waiting for
   confirmation. **The user must press a button on the unit**; it then
   sends `07/46` payload `01 00` (approved), which the app acknowledges
   (flags `0xc0`, same sequence number, payload `00`). Without the press the
   unit stays at `00 02`, refuses step 6 and drops the link after ~14 s. A
   known app id doesn't skip the confirmation.
5. `07/18` payload `41 44 00 00`: switch Wi-Fi on, with the phone's country
   code (`AD` here). Answer `00`.
6. `07/07` payload `00` → `00 | len | SSID`; `07/0e` payload `00` →
   `00 | len | WPA2 passphrase` (8 digits). Both answer `ff 00` if not paired.
7. **Disconnect BLE right away** (Mimo does it 2 ms after step 6). The access
   point only comes up once the BLE link is gone: while connected it never
   appears. It's a visible WPA2 network on 5 GHz (channel 36 here), BSSID =
   the BLE address + 1, and hands out 192.168.0.x addresses with the unit at
   192.168.0.1.

Then, on the access point (captured from Mimo with `rvictl` and reproduced
from a Mac; files came out byte-identical to the USB copies, 3.7 MB/s):

8. The unit broadcasts UDP `01 00 00 00` to `255.255.255.255:10001` about
   once a second and serves DHCP (Mimo got 192.168.0.2, the Mac .3).
9. **TCP 192.168.0.1:60000** carries raw DUML frames, no extra header — the
   same protocol as USB and BLE. Mimo sends `00/00` payload `01 00` (ping,
   answered `01 00`) every 0.5 s and the `00/2b` heartbeat to `0xf0`, plus
   status polls (`5b/12`, `5b/09`, `04/1f`).
10. **`5b/19` payload `01 01 00 01`** (answer `00`) starts the file server.
    Before it, port 80 refuses connections.
11. **HTTP on port 80** (plain HTTP/1.1, `Accept-Ranges: bytes`, ETags,
    `Date` stuck at 1969):
    - `GET /data/emmc0/files_info.json` →
      `{"name": "TX RECORD FILES INFO", "files": [{"fname":
      "data/emmc0/TX_MIC002_…/TX01_MIC003_…_orig.wav", "fsize": 13576258,
      "duration_sec": 94.1, "date": 20261123232008}, …]}`
    - `GET /<fname>` → the WAV as stored (`Content-Type: audio/x-wav`),
      the same bytes as over USB mass storage.

On macOS, seeing or joining the network from a script needs Location
permission (SSIDs are redacted without it), and the AP shares the home
network's 192.168.0.0/24, so traffic must be bound to the Wi-Fi interface.

### Receiver over Wi-Fi (control)

The receiver pairs the same way, with three differences:

- It advertises over BLE once **Connect to App** is chosen in the control
  menu (swipe down on the home screen). While its access point is up it
  stays silent over BLE, even after the app that opened it has gone.
- Step 4's confirmation happens on its touch screen: it shows the app's
  4-digit PIN as a verification code and the user taps to accept.
- On its access point (`DJI Mic 3 RX-63d7b0`), TCP 60000 carries **exactly
  the USB frames**: the receiver's status heartbeat (`5b/03`, a `0x26`
  receiver record, byte-for-byte the receiver-only USB frame), identity
  pushes and command ACKs. Setting commands are the USB ones (address
  `0x5a`), and the same `00/00` ping keeps the session open.

**In this mode the receiver drops its transmitters**: they show as
disconnected on its own screen and no transmitter slots or levels arrive
(DJI notes the receiver doesn't record while connected to the app). So the
link is good for the receiver's own settings, not for monitoring.

A unit's access-point passphrase doesn't change between sessions (seen on
the receiver and a transmitter, from Mimo and from a Mac), so an app can
remember it and rejoin a unit whose access point is still up without going
through Bluetooth again.

---

## Receiver families and model detection

| USB id | Interface | Receivers | How the app tells them apart |
| --- | --- | --- | --- |
| `2ca3:4011` | 6 (bulk `0x06`/`0x86`) | DJI Mic Mini, DJI Mic Series Mobile Receiver | The Mobile RX reports the product name `DJI Mic Mini 2` for itself (identity record `0x06`, unit 0); the Mic Mini reports `DJI Mic Mini` |
| `2ca3:4015`, `4115` | 4 alt 1 (bulk `0x01`/`0x81`) | DJI Mic 3, DJI Mic Mini 2S | "Get version" hardware name: `DM303_RX` is the Mic 3; any other answer (or an RX name containing "Mini 2S") is the Mini 2S |

The app opens a receiver with the model matched by USB id and switches to
the right one as soon as the receiver identifies itself; the transport is the
same within a family, so the open interface is kept.

### Mobile Receiver (sources: upstream PR #4, piedol/lavdeck — both hardware-tested)

- Receiver gain `0x0039` (receiver target, −12…+12 in 6 dB steps), status
  offset 22; monitoring gain `0x0026` (1 dB steps), status offset 27.
- Noise cancelling is set by the O/L/H switch on the receiver. Its state is
  read from the first connected transmitter's slot (`+7` bit `0x01` on,
  `+6` bit `0x20` strong); writes are refused.
- Bus-powered: no auto-off and no camera power sync.
- It accepts Mic Mini, Mini 2, Mini 2S, Mic 2 and Mic 3 transmitters.

### Transmitter capabilities by model

Transmitters report a product name in the identity push. The 32-byte status
slot has the same layout on all of them; which fields are meaningful depends
on the model:

| Transmitter | Extra fields and commands |
| --- | --- |
| `DJI Mic Mini` | none beyond the Mic Mini table above |
| `DJI Mic Mini 2` | voice tone |
| `DJI Mic 2` | mute (`0x0009`, per TX, `+6` bit `0x04`), internal recording (`0x0002`, per TX, `+9` bit `0x10`), 32-bit float, vibration, REC button and record on power-on (broadcast), recording time (`+14`/`+16`). Voice tone and gain are ACKed but don't change the status, so they aren't offered. Noise-cancel writes are rejected by the Mobile RX. Source: nicolasmmb/DJI-Mic-Utility (hardware-tested) |
| `DJI Mic Mini 2S`, `DJI Mic 3` | everything above plus voice tone, input gain (`0x0039` per TX, `+13`) and loop recording |

### DJI Mic Mini 2S receiver (sources: usokawa/dji-mic-mo, kroqdotdev/lavboard)

Same interface, framing and status layout as the Mic 3 receiver, without
lossless audio, the environment setting or the receiver/monitoring gains.
Differences in settings:

| Setting | Cmd | Values | Readback |
| --- | --- | --- | --- |
| Safety Track | `0x0021` | `0x00` off, `0x01` on | header offset 48 bit `0x40` |
| Automatic recording | `0x003e` | `0x00` off, `0x01` on power-on, `0x02` with the receiver | first TX slot `+31` bits `0x10` / `0x20` |
| Dual-file recording | `0x003d` | `0x00` off, `0x02` on | first TX slot `+31` bit `0x80` |
| Low-battery auto record | `0x002b` | as on the Mic 3 | `+11` bit `0x10` |

Not tested on hardware by this project.

### Not supported

No USB control interface has been documented for the DJI Mic 2's own
receiver or for the original DJI Mic (2021). The Mic 2's USB ids seen in the
wild (`2ca3:002d`, `2ca3:0a69`) are file-transfer (MTP) modes.
