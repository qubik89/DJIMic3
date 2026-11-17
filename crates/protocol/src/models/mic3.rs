//! DJI Mic 3 model definition.
//!
//! The Mic 3 receiver speaks the same v2 DUML dialect as the DJI Mic Mini
//! (see `PROTOCOL.md`), but exposes it on a different USB interface: the
//! `com.dji.mic` External Accessory interface (number 4), whose bulk endpoints
//! only exist on alternate setting 1. No iAP handshake is needed — the
//! receiver starts streaming status frames as soon as that alternate setting
//! is selected.
//!
//! Its status push shares the Mic Mini's v2 layout and adds receiver fields
//! the Mini doesn't have (four-channel mode, a software output gain, a
//! monitoring gain). The receiver rejects the Mini's receiver-level Safety
//! Track and Clipping Control commands, so they aren't offered here.

use super::{mic_mini, DeviceModel};
use crate::packet;
use crate::settings::{Setting, SettingKind, SettingOption, V2Target};
use crate::state::{DeviceStatus, MAX_TX};

/// The DJI Mic 3 (receiver + transmitters over USB).
pub struct Mic3;

const MODEL_ID: &str = "dji-mic-3";

const TARGET_RX: V2Target = V2Target::Fixed(0x0000);
const TARGET_TX: V2Target = V2Target::Fixed(0xffff);
/// One specific transmitter, chosen by the caller.
const TARGET_TX_ONE: V2Target = V2Target::Tx;

const fn off_on() -> [SettingOption; 2] {
    [
        SettingOption { value: "off", label: "Off", wire: 0x00 },
        SettingOption { value: "on", label: "On", wire: 0x01 },
    ]
}

static OFF_ON: [SettingOption; 2] = off_on();
static NC_OPTS: [SettingOption; 2] = [
    SettingOption { value: "basic", label: "Basic", wire: 0x00 },
    SettingOption { value: "strong", label: "Strong", wire: 0x01 },
];
static AUDIO_CHANNEL_OPTS: [SettingOption; 3] = [
    SettingOption { value: "mono", label: "Mono", wire: 0x00 },
    SettingOption { value: "stereo", label: "Stereo", wire: 0x02 },
    SettingOption { value: "quad", label: "4CH", wire: 0x04 },
];
// Wire values are the gain in dB as a signed byte. Slugs avoid a leading
// `-` so they can be passed on the command line without `--`.
pub(super) static RX_GAIN_OPTS: [SettingOption; 5] = [
    SettingOption { value: "m12", label: "−12", wire: 0xf4 },
    SettingOption { value: "m6", label: "−6", wire: 0xfa },
    SettingOption { value: "0", label: "0", wire: 0x00 },
    SettingOption { value: "p6", label: "+6", wire: 0x06 },
    SettingOption { value: "p12", label: "+12", wire: 0x0c },
];
pub(super) static GAIN_1DB_OPTS: [SettingOption; 25] = [
    SettingOption { value: "m12", label: "-12 dB", wire: 0xf4 },
    SettingOption { value: "m11", label: "-11 dB", wire: 0xf5 },
    SettingOption { value: "m10", label: "-10 dB", wire: 0xf6 },
    SettingOption { value: "m9", label: "-9 dB", wire: 0xf7 },
    SettingOption { value: "m8", label: "-8 dB", wire: 0xf8 },
    SettingOption { value: "m7", label: "-7 dB", wire: 0xf9 },
    SettingOption { value: "m6", label: "-6 dB", wire: 0xfa },
    SettingOption { value: "m5", label: "-5 dB", wire: 0xfb },
    SettingOption { value: "m4", label: "-4 dB", wire: 0xfc },
    SettingOption { value: "m3", label: "-3 dB", wire: 0xfd },
    SettingOption { value: "m2", label: "-2 dB", wire: 0xfe },
    SettingOption { value: "m1", label: "-1 dB", wire: 0xff },
    SettingOption { value: "0", label: "0 dB", wire: 0x00 },
    SettingOption { value: "p1", label: "+1 dB", wire: 0x01 },
    SettingOption { value: "p2", label: "+2 dB", wire: 0x02 },
    SettingOption { value: "p3", label: "+3 dB", wire: 0x03 },
    SettingOption { value: "p4", label: "+4 dB", wire: 0x04 },
    SettingOption { value: "p5", label: "+5 dB", wire: 0x05 },
    SettingOption { value: "p6", label: "+6 dB", wire: 0x06 },
    SettingOption { value: "p7", label: "+7 dB", wire: 0x07 },
    SettingOption { value: "p8", label: "+8 dB", wire: 0x08 },
    SettingOption { value: "p9", label: "+9 dB", wire: 0x09 },
    SettingOption { value: "p10", label: "+10 dB", wire: 0x0a },
    SettingOption { value: "p11", label: "+11 dB", wire: 0x0b },
    SettingOption { value: "p12", label: "+12 dB", wire: 0x0c },
];
// Inverted on the wire relative to every other toggle: off = 1, on = 2.
static LOUDNESS_OPTS: [SettingOption; 2] = [
    SettingOption { value: "off", label: "Off", wire: 0x01 },
    SettingOption { value: "on", label: "On", wire: 0x02 },
];
static VOICE_TONE_OPTS: [SettingOption; 3] = [
    SettingOption { value: "standard", label: "Standard", wire: 0x00 },
    SettingOption { value: "rich", label: "Rich", wire: 0x01 },
    SettingOption { value: "bright", label: "Bright", wire: 0x02 },
];
pub(super) static RECORD_OPTS: [SettingOption; 2] = [
    SettingOption { value: "off", label: "Stopped", wire: 0x00 },
    SettingOption { value: "on", label: "Recording", wire: 0x01 },
];
static LED_OPTS: [SettingOption; 2] = [
    SettingOption { value: "off", label: "Off", wire: 0x02 },
    SettingOption { value: "on", label: "On", wire: 0x00 },
];

const G_AUDIO: &str = "Audio";
const G_POWER: &str = "Power & Startup";
const G_DEVICE: &str = "Device";
const G_TX: &str = "Transmitter";
const G_REC: &str = "Internal Recording";

const fn setting(
    id: &'static str,
    label: &'static str,
    group: &'static str,
    kind: SettingKind,
    command: u16,
    target: V2Target,
    options: &'static [SettingOption],
    note: Option<&'static str>,
) -> Setting {
    Setting {
        id,
        label,
        group,
        kind,
        // Mic 3 firmware only speaks v2, but the ids double as "exists on
        // v1" markers for front-ends, so mirror the v2 id.
        v1_command: Some(command),
        v2_command: command,
        v2_target: target,
        options,
        exclusive_with: &[],
        note,
        locked: None,
    }
}

pub(super) static SETTINGS: [Setting; 25] = [
    setting("noise-cancel", "Noise Cancel", G_AUDIO, SettingKind::Enum, 0x0037, TARGET_TX, &NC_OPTS, None),
    setting("noise-cancel-power", "Noise Cancelling", G_AUDIO, SettingKind::Toggle, 0x0038, TARGET_TX, &OFF_ON, None),
    setting("noise-cancel-button", "Toggle via TX power button", G_AUDIO, SettingKind::Toggle, 0x000f, TARGET_TX, &OFF_ON, None),
    setting("low-cut", "Low Cut", G_AUDIO, SettingKind::Toggle, 0x0003, TARGET_TX, &OFF_ON, None),
    setting(
        "stereo",
        "Channels",
        G_AUDIO,
        SettingKind::Enum,
        0x0008,
        TARGET_RX,
        &AUDIO_CHANNEL_OPTS,
        Some("Switching to or from 4CH reconnects the receiver's USB audio."),
    ),
    setting("rx-gain", "RX Gain", G_AUDIO, SettingKind::Enum, 0x0039, TARGET_RX, &RX_GAIN_OPTS, None),
    setting("lossless", "Lossless Audio", G_AUDIO, SettingKind::Toggle, 0x0027, TARGET_RX, &OFF_ON, None),
    setting("monitor-gain", "Monitoring Gain", G_AUDIO, SettingKind::Select, 0x0026, TARGET_RX, &GAIN_1DB_OPTS, None),
    setting("tx-clip-control", "Clipping Control", G_AUDIO, SettingKind::Toggle, 0x0024, TARGET_TX, &OFF_ON, None),
    setting("loudness-balance", "Loudness Balance", G_AUDIO, SettingKind::Toggle, 0x002c, TARGET_TX, &LOUDNESS_OPTS, None),
    setting("auto-off-15m", "Auto Off RX (15 min)", G_POWER, SettingKind::Toggle, 0x0010, TARGET_RX, &OFF_ON, None),
    setting("tx-auto-off-15m", "Auto Off TX (15 min)", G_POWER, SettingKind::Toggle, 0x0010, TARGET_TX, &OFF_ON, None),
    setting("camera-power", "RX On/Off follows Camera", G_POWER, SettingKind::Toggle, 0x0020, TARGET_RX, &OFF_ON, None),
    setting(
        "plug-free",
        "Plug-Free Ext. Speaker",
        G_DEVICE,
        SettingKind::Toggle,
        0x0023,
        TARGET_RX,
        &OFF_ON,
        Some("Changing this reboots the receiver."),
    ),
    setting("mic-leds", "Mic LEDs", G_DEVICE, SettingKind::Toggle, 0x000a, TARGET_TX, &LED_OPTS, None),
    // The transmitters apply these to every unit even when addressed to
    // one, so they're broadcast like the other shared TX settings.
    setting("vibration", "TX Vibration", G_DEVICE, SettingKind::Toggle, 0x0004, TARGET_TX, &OFF_ON, None),
    setting("float32", "32-bit Float", G_REC, SettingKind::Toggle, 0x000c, TARGET_TX, &OFF_ON, None),
    setting("loop-recording", "Loop Recording", G_REC, SettingKind::Toggle, 0x002a, TARGET_TX, &OFF_ON, None),
    setting("startup-recording", "Record on Power-On", G_REC, SettingKind::Toggle, 0x002e, TARGET_TX, &OFF_ON, None),
    setting("low-power-recording", "Low-Battery Auto Record", G_REC, SettingKind::Toggle, 0x002b, TARGET_TX, &OFF_ON, None),
    setting("rec-stop", "Record via TX Button", G_REC, SettingKind::Toggle, 0x000b, TARGET_TX, &OFF_ON, None),
    // Addressed to one transmitter each; front-ends render these per TX.
    setting("mute", "Mute", G_TX, SettingKind::Toggle, 0x0009, TARGET_TX_ONE, &OFF_ON, None),
    setting("record", "Recording", G_TX, SettingKind::Toggle, 0x0002, TARGET_TX_ONE, &RECORD_OPTS, None),
    setting("voice-tone", "Voice Tone", G_TX, SettingKind::Enum, 0x0029, TARGET_TX_ONE, &VOICE_TONE_OPTS, None),
    setting("tx-gain", "Gain", G_TX, SettingKind::Select, 0x0039, TARGET_TX_ONE, &GAIN_1DB_OPTS, None),
];

/// Offset 14 of a v2 push names its kind: `0x03` is the status push.
const PUSH_KIND_OFFSET: usize = 14;
const PUSH_KIND_STATUS: u8 = 0x03;
/// Offset 20: `0x01` environment = outdoors, `0x04` lossless audio, `0x80`
/// on/off follows camera.
const RX_FLAGS_OFFSET: usize = 20;
/// Offset 21: `0x04` stereo, `0x08` four-channel (neither = mono).
const CHANNEL_FLAGS_OFFSET: usize = 21;
/// Offset 22: receiver output gain, signed dB.
const RX_GAIN_OFFSET: usize = 22;
/// Offset 27: monitoring (headphone) gain, signed dB.
const MONITOR_GAIN_OFFSET: usize = 27;
/// Status header length and per-transmitter slot length (see `PROTOCOL.md`).
const HEADER_LEN: usize = 52;
const TX_SLOT_LEN: usize = 32;

fn on_off(b: bool) -> String {
    if b { "on" } else { "off" }.to_string()
}

fn u16_le(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

/// Local wall-clock time as `[year % 100, month, day, hour, minute, second]`.
fn local_time() -> [u8; 6] {
    use chrono::{Datelike, Local, Timelike};
    let now = Local::now();
    [
        (now.year() % 100) as u8,
        now.month() as u8,
        now.day() as u8,
        now.hour() as u8,
        now.minute() as u8,
        now.second() as u8,
    ]
}

/// Set the transmitters' clock (command `0x0033`, receiver target) so
/// internal recordings get correct file timestamps.
pub(crate) fn clock_sync_packet(seq: u16, t: &[u8; 6]) -> Vec<u8> {
    let mut p = vec![0x55, 0x1d, 0x04, 0x00];
    p[3] = packet::header_crc8(&p[..3]);
    p.extend_from_slice(&[0x02, 0x5a, seq as u8, (seq >> 8) as u8]);
    p.extend_from_slice(&[0x40, 0x5b, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x33, 0x00, 0x08, 0x09]);
    p.extend_from_slice(t);
    p.push(0x00);
    crate::crc::append(p, packet::Dialect::V2.crc_init(), packet::Dialect::V2.crc_residue())
}

/// The 1 dB gain slug for a signed-dB status byte.
pub(super) fn gain_slug(wire: u8) -> Option<String> {
    option_slug(&GAIN_1DB_OPTS, wire)
}

pub(super) fn option_slug(options: &[SettingOption], wire: u8) -> Option<String> {
    options.iter().find(|o| o.wire == wire).map(|o| o.value.to_string())
}

impl DeviceModel for Mic3 {
    fn id(&self) -> &'static str {
        MODEL_ID
    }
    fn name(&self) -> &'static str {
        "DJI Mic 3"
    }
    fn pictogram_key(&self) -> &'static str {
        "mic-3"
    }
    fn usb_ids(&self) -> &'static [(u16, u16)] {
        // The receiver re-enumerates as 0x4115 while in four-channel mode.
        &[(0x2ca3, 0x4015), (0x2ca3, 0x4115)]
    }
    fn interface(&self) -> u8 {
        4
    }
    fn alt_setting(&self) -> u8 {
        1
    }
    fn max_tx(&self) -> usize {
        MAX_TX
    }
    fn connect_commands(&self, seq: u16, _dialect: packet::Dialect) -> Vec<Vec<u8>> {
        vec![packet::build_version_request(seq), clock_sync_packet(seq.wrapping_add(1), &local_time())]
    }
    fn bulk_in(&self) -> u8 {
        0x81
    }
    fn bulk_out(&self) -> u8 {
        0x01
    }
    fn settings(&self) -> &'static [Setting] {
        &SETTINGS
    }

    fn refine(&self, status: &DeviceStatus) -> Option<&'static dyn DeviceModel> {
        (family_variant(status) == Variant::Mini2S).then_some(&MicMini2S as &'static dyn DeviceModel)
    }

    fn decode(&self, previous: &DeviceStatus, f: &[u8]) -> Option<DeviceStatus> {
        decode_family(previous, f, Variant::Mic3)
    }
}

/// The two receivers on this interface share USB ids; the hardware name
/// from the version request tells them apart (`DM303_RX` = Mic 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variant {
    Mic3,
    Mini2S,
}

const MIC3_HARDWARE: &str = "DM303_RX";

fn family_variant(status: &DeviceStatus) -> Variant {
    let rx = status.rx.as_ref();
    let hardware = rx.and_then(|r| r.hardware.as_deref());
    let name = rx.and_then(|r| r.product_name.as_deref());
    match (hardware, name) {
        (Some(MIC3_HARDWARE), _) => Variant::Mic3,
        (_, Some(n)) if n.contains("Mini 2S") => Variant::Mini2S,
        (Some(_), _) => Variant::Mini2S,
        // Not identified yet: stay as the Mic 3, the default for this id.
        (None, _) => Variant::Mic3,
    }
}

fn decode_family(previous: &DeviceStatus, f: &[u8], variant: Variant) -> Option<DeviceStatus> {
    // Firmware on this interface only ever speaks v2.
    if packet::heartbeat_dialect(f)? != packet::Dialect::V2 {
        return None;
    }
    let mut status = mic_mini::decode_v2(previous, f)?;
    status.model_id = match variant {
        Variant::Mic3 => MODEL_ID,
        Variant::Mini2S => MINI2S_ID,
    }
    .into();
    status.protocol_version = Some(2);

    if f.get(PUSH_KIND_OFFSET) == Some(&PUSH_KIND_STATUS) && f.len() > MONITOR_GAIN_OFFSET + 2 {
        // No physical gain dial on these receivers.
        status.gain_dial = None;
        let s = &mut status.settings;
        // The Mic 3 rejects both; the Mini 2S keeps Safety Track.
        s.remove("clip-limiter");
        if variant == Variant::Mic3 {
            s.remove("safety-track");
        }
        let channels = f[CHANNEL_FLAGS_OFFSET];
        let mode = if channels & 0x08 != 0 {
            "quad"
        } else if channels & 0x04 != 0 {
            "stereo"
        } else {
            "mono"
        };
        s.insert("stereo".into(), mode.into());
        let flags = f[RX_FLAGS_OFFSET];
        if variant == Variant::Mic3 {
            if let Some(v) = option_slug(&RX_GAIN_OPTS, f[RX_GAIN_OFFSET]) {
                s.insert("rx-gain".into(), v);
            }
            if let Some(v) = option_slug(&GAIN_1DB_OPTS, f[MONITOR_GAIN_OFFSET]) {
                s.insert("monitor-gain".into(), v);
            }
            s.insert("lossless".into(), on_off(flags & 0x04 != 0));
        }

        // Receiver battery: offset 21 bit 0x10 charging, bits 0xe0 a
        // 1 (full) … 7 (empty) gauge like the transmitters'.
        if let Some(rx) = status.rx.as_mut() {
            if variant == Variant::Mic3 {
                rx.environment =
                    Some(if flags & 0x01 != 0 { "outdoors" } else { "indoors" }.to_string());
            }
            rx.charging = Some(channels & 0x10 != 0);
            rx.battery = Some((channels >> 5) & 0x07).filter(|&b| b != 0);
        }

        // Transmitter slots. Shared settings mirror across every
        // transmitter, so the first slot stands for all of them.
        let tx_count = (f.len() - HEADER_LEN - 2) / TX_SLOT_LEN;
        for pos in 0..tx_count.min(MAX_TX) {
            let slot = &f[HEADER_LEN + TX_SLOT_LEN * pos..HEADER_LEN + TX_SLOT_LEN * (pos + 1)];
            if pos == 0 {
                let s = &mut status.settings;
                s.insert("tx-clip-control".into(), on_off(slot[8] & 0x04 != 0));
                s.insert("loudness-balance".into(), on_off(slot[11] & 0x80 != 0));
                s.insert("vibration".into(), on_off(slot[9] & 0x01 != 0));
                s.insert("float32".into(), on_off(slot[9] & 0x08 != 0));
                s.insert("loop-recording".into(), on_off(slot[11] & 0x08 != 0));
                s.insert("low-power-recording".into(), on_off(slot[11] & 0x10 != 0));
                s.insert("rec-stop".into(), on_off(slot[11] & 0x20 != 0));
                match variant {
                    Variant::Mic3 => {
                        s.insert("startup-recording".into(), on_off(slot[8] & 0x80 != 0));
                    }
                    Variant::Mini2S => {
                        // Slot +31: 0x10 record on power-on, 0x20 record
                        // with the receiver, 0x80 dual-file recording.
                        let mode = if slot[31] & 0x10 != 0 {
                            "startup"
                        } else if slot[31] & 0x20 != 0 {
                            "receiver"
                        } else {
                            "off"
                        };
                        s.remove("startup-recording");
                        s.insert("auto-record".into(), mode.into());
                        s.insert("dual-file".into(), on_off(slot[31] & 0x80 != 0));
                    }
                }
            }
            let Some(i) = mic_mini::tx_slot(slot[1]) else { continue };
            let Some(tx) = status.tx[i].as_mut() else { continue };
            // Mic 3 transmitters don't report a name; on the Mini 2S
            // receiver an unnamed transmitter is most likely a Mini 2S.
            let caps = match variant {
                Variant::Mic3 => mic_mini::TX_CAPS_FULL,
                Variant::Mini2S => mic_mini::tx_caps(tx.product_name.as_deref()).unwrap_or(mic_mini::TX_CAPS_FULL),
            };
            let t = &mut tx.settings;
            t.clear();
            if caps.voice_tone {
                if let Some(tone) = tx.voice_tone.clone() {
                    t.insert("voice-tone".into(), tone);
                }
            }
            if caps.gain {
                if let Some(v) = option_slug(&GAIN_1DB_OPTS, slot[13]) {
                    t.insert("tx-gain".into(), v);
                }
            }
            if caps.extended {
                tx.rec_total = Some(u16_le(&slot[14..16]));
                tx.rec_remaining = Some(u16_le(&slot[16..18]));
                t.insert("record".into(), on_off(slot[9] & 0x10 != 0));
                t.insert("mute".into(), on_off(slot[6] & 0x04 != 0));
            }
        }
    }
    Some(status)
}

/// The DJI Mic Mini 2S receiver: same interface and protocol as the Mic 3,
/// with Safety Track and the Mini 2S recording modes, and without the
/// Mic 3's lossless, environment and receiver/monitoring gain controls.
/// Settings follow usokawa/dji-mic-mo and kroqdotdev/lavboard; not tested
/// on hardware by this project.
pub struct MicMini2S;

const MINI2S_ID: &str = "dji-mic-mini-2s";

static AUTO_RECORD_OPTS: [SettingOption; 3] = [
    SettingOption { value: "off", label: "Off", wire: 0x00 },
    SettingOption { value: "startup", label: "On power-on", wire: 0x01 },
    SettingOption { value: "receiver", label: "With receiver", wire: 0x02 },
];
static DUAL_FILE_OPTS: [SettingOption; 2] = [
    SettingOption { value: "off", label: "Off", wire: 0x00 },
    SettingOption { value: "on", label: "On", wire: 0x02 },
];

fn mini2s_settings() -> &'static [Setting] {
    static CELL: std::sync::OnceLock<Vec<Setting>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
        let mut out = Vec::new();
        for s in SETTINGS.iter() {
            match s.id {
                "lossless" | "rx-gain" | "monitor-gain" | "startup-recording" => {}
                "stereo" => {
                    let mut st = s.clone();
                    st.exclusive_with = &["safety-track"];
                    out.push(st);
                    let mut safety = setting("safety-track", "Safety Track", G_AUDIO, SettingKind::Toggle, 0x0021, TARGET_RX, &OFF_ON, None);
                    safety.exclusive_with = &["stereo"];
                    out.push(safety);
                }
                "rec-stop" => {
                    out.push(s.clone());
                    out.push(setting("auto-record", "Auto Recording", G_REC, SettingKind::Enum, 0x003e, TARGET_TX, &AUTO_RECORD_OPTS, None));
                    out.push(setting("dual-file", "Dual-File Recording", G_REC, SettingKind::Toggle, 0x003d, TARGET_TX, &DUAL_FILE_OPTS, None));
                }
                _ => out.push(s.clone()),
            }
        }
        out
    })
}

impl DeviceModel for MicMini2S {
    fn id(&self) -> &'static str {
        MINI2S_ID
    }
    fn name(&self) -> &'static str {
        "DJI Mic Mini 2S"
    }
    fn pictogram_key(&self) -> &'static str {
        "mic-mini-2s"
    }
    fn usb_ids(&self) -> &'static [(u16, u16)] {
        Mic3.usb_ids()
    }
    fn interface(&self) -> u8 {
        Mic3.interface()
    }
    fn alt_setting(&self) -> u8 {
        Mic3.alt_setting()
    }
    fn max_tx(&self) -> usize {
        MAX_TX
    }
    fn connect_commands(&self, seq: u16, dialect: packet::Dialect) -> Vec<Vec<u8>> {
        Mic3.connect_commands(seq, dialect)
    }
    fn bulk_in(&self) -> u8 {
        Mic3.bulk_in()
    }
    fn bulk_out(&self) -> u8 {
        Mic3.bulk_out()
    }
    fn settings(&self) -> &'static [Setting] {
        mini2s_settings()
    }
    fn refine(&self, status: &DeviceStatus) -> Option<&'static dyn DeviceModel> {
        (family_variant(status) == Variant::Mic3).then_some(&Mic3 as &'static dyn DeviceModel)
    }
    fn decode(&self, previous: &DeviceStatus, f: &[u8]) -> Option<DeviceStatus> {
        decode_family(previous, f, Variant::Mini2S)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Captured from a Mic 3 receiver with no transmitter connected.
    const STATUS_0TX: [u8; 54] = [
        0x55, 0x36, 0x04, 0x3d, 0x5a, 0x02, 0xb4, 0xd5, 0x00, 0x5b, 0x03, 0x03, 0x26, 0x00, 0x03,
        0x00, 0x00, 0x00, 0x00, 0x20, 0x80, 0x51, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x00,
        0x19, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x03, 0x03, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xdd, 0xe2,
    ];

    // Captured with one transmitter (TX1) connected.
    const STATUS_1TX: [u8; 86] = [
        0x55, 0x56, 0x04, 0x67, 0x5a, 0x02, 0x46, 0xb1, 0x00, 0x5b, 0x03, 0x03, 0x46, 0x00, 0x03,
        0x00, 0x00, 0x00, 0x00, 0x20, 0x80, 0x51, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x00,
        0x19, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x03, 0x03, 0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x01, 0x00, 0x00, 0x00, 0x1a, 0x90, 0x24,
        0x04, 0x21, 0x00, 0x78, 0x50, 0x00, 0x3d, 0x02, 0x3d, 0x02, 0x90, 0x00, 0x19, 0x01, 0x01,
        0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x03, 0x00, 0x00, 0x5d, 0x0e,
    ];

    fn decode(frame: &[u8]) -> DeviceStatus {
        Mic3.decode(&DeviceStatus::disconnected(MODEL_ID), frame).expect("decodes")
    }

    #[test]
    fn decodes_receiver_only_status() {
        let st = decode(&STATUS_0TX);
        assert_eq!(st.model_id, MODEL_ID);
        assert!(st.connected);
        assert_eq!(st.tx_count(), 0);
        assert_eq!(st.settings["stereo"], "mono");
        assert_eq!(st.settings["rx-gain"], "0");
        assert_eq!(st.settings["monitor-gain"], "0");
        assert_eq!(st.settings["auto-off-15m"], "on");
        assert_eq!(st.settings["camera-power"], "on");
        assert!(!st.settings.contains_key("safety-track"));
        assert!(!st.settings.contains_key("clip-limiter"));
    }

    #[test]
    fn decodes_channel_mode_and_gains() {
        let mut f = STATUS_0TX;
        f[CHANNEL_FLAGS_OFFSET] = 0x59; // four-channel
        f[RX_GAIN_OFFSET] = 0x06;
        f[MONITOR_GAIN_OFFSET] = 0xfd;
        let st = decode(&f);
        assert_eq!(st.settings["stereo"], "quad");
        assert_eq!(st.settings["rx-gain"], "p6");
        assert_eq!(st.settings["monitor-gain"], "m3");

        f[CHANNEL_FLAGS_OFFSET] = 0x55;
        assert_eq!(decode(&f).settings["stereo"], "stereo");
    }

    #[test]
    fn decodes_transmitter_slot() {
        let st = decode(&STATUS_1TX);
        assert_eq!(st.tx_count(), 1);
        assert_eq!(st.settings["low-cut"], "on");
        assert_eq!(st.settings["tx-clip-control"], "on");
        assert_eq!(st.settings["loudness-balance"], "off");
        assert_eq!(st.settings["float32"], "off");
        assert_eq!(st.settings["loop-recording"], "on");
        assert_eq!(st.settings["low-power-recording"], "on");
        assert_eq!(st.settings["vibration"], "on");
        assert_eq!(st.settings["startup-recording"], "off");
        assert_eq!(st.settings["rec-stop"], "on");
        assert_eq!(st.rx.as_ref().unwrap().battery, Some(2));
        assert_eq!(st.rx.as_ref().unwrap().charging, Some(true));
        assert_eq!(st.rx.as_ref().unwrap().environment.as_deref(), Some("indoors"));
        assert_eq!(st.settings["lossless"], "off");
        assert_eq!(st.tx[0].as_ref().unwrap().settings["mute"], "off");
        let tx = st.tx[0].as_ref().unwrap();
        assert_eq!(tx.battery, Some(1));
        assert_eq!(tx.rec_total, Some(573));
        assert_eq!(tx.rec_remaining, Some(573));
        assert_eq!(tx.settings["voice-tone"], "standard");
        assert_eq!(tx.settings["tx-gain"], "0");
        assert_eq!(tx.settings["record"], "off");

        let mut f = STATUS_1TX;
        f[52 + 9] = 0xb8; // recording, 32-bit float, bright tone, vibration off
        f[52 + 13] = 0xfd; // -3 dB
        f[52 + 11] = 0xa0; // loudness balance on, loop/low-power off
        let st = decode(&f);
        let tx = st.tx[0].as_ref().unwrap();
        assert_eq!(st.settings["loudness-balance"], "on");
        assert_eq!(tx.settings["voice-tone"], "bright");
        assert_eq!(tx.settings["tx-gain"], "m3");
        assert_eq!(tx.settings["record"], "on");
        assert_eq!(st.settings["float32"], "on");
        assert_eq!(st.settings["loop-recording"], "off");
        assert_eq!(st.settings["low-power-recording"], "off");
        assert_eq!(st.settings["vibration"], "off");
    }

    fn with_hardware(name: &str) -> DeviceStatus {
        let mut st = DeviceStatus::disconnected(MODEL_ID);
        st.rx = Some(crate::state::RxInfo { hardware: Some(name.into()), ..crate::state::RxInfo::unknown() });
        st
    }

    #[test]
    fn hardware_name_tells_mic3_and_mini_2s_apart() {
        assert!(Mic3.refine(&DeviceStatus::disconnected(MODEL_ID)).is_none(), "unidentified stays Mic 3");
        assert!(Mic3.refine(&with_hardware("DM303_RX")).is_none());
        assert_eq!(Mic3.refine(&with_hardware("WM2S_RX")).map(|m| m.id()), Some(MINI2S_ID));
        assert_eq!(MicMini2S.refine(&with_hardware("DM303_RX")).map(|m| m.id()), Some(MODEL_ID));
        assert!(MicMini2S.refine(&with_hardware("WM2S_RX")).is_none());
    }

    #[test]
    fn mini_2s_settings_and_recording_modes() {
        let ids: Vec<_> = MicMini2S.settings().iter().map(|s| s.id).collect();
        for id in ["safety-track", "auto-record", "dual-file", "stereo", "tx-gain", "mute", "record"] {
            assert!(ids.contains(&id), "{id}");
        }
        for id in ["lossless", "rx-gain", "monitor-gain", "startup-recording"] {
            assert!(!ids.contains(&id), "{id}");
        }
        let pkt = MicMini2S.build_command(1, packet::Dialect::V2, "auto-record", "receiver", None).unwrap();
        assert_eq!(&pkt[16..20], &[0x3e, 0x00, 0x01, 0x02]);

        let mut f = STATUS_1TX;
        f[52 + 31] = 0xa0; // with receiver + dual-file
        let st = MicMini2S.decode(&DeviceStatus::disconnected(MINI2S_ID), &f).unwrap();
        assert_eq!(st.model_id, MINI2S_ID);
        assert_eq!(st.settings["auto-record"], "receiver");
        assert_eq!(st.settings["dual-file"], "on");
        assert!(st.settings.contains_key("safety-track"));
        assert!(!st.settings.contains_key("lossless"));
        assert!(!st.settings.contains_key("startup-recording"));
        assert_eq!(st.rx.as_ref().unwrap().environment, None);
    }

    #[test]
    fn clock_sync_matches_reference_capture() {
        // Sent to the receiver on 2026-11-17 12:11:43 and acknowledged.
        let p = clock_sync_packet(0x0109, &[0x1a, 0x0b, 0x11, 0x0c, 0x0b, 0x2b]);
        assert_eq!(p.len(), 29);
        assert_eq!(&p[..4], &[0x55, 0x1d, 0x04, packet::header_crc8(&[0x55, 0x1d, 0x04])]);
        assert_eq!(&p[16..27], &[0x33, 0x00, 0x08, 0x09, 0x1a, 0x0b, 0x11, 0x0c, 0x0b, 0x2b, 0x00]);
        assert_eq!(packet::header_crc8(&[0x55, 0x16, 0x04]), 0xfc); // matches the 22-byte command header
    }

    #[test]
    fn every_setting_builds_a_v2_command() {
        for s in Mic3.settings() {
            for o in s.options {
                let tx = (s.v2_target == V2Target::Tx).then_some(0);
                let pkt = Mic3
                    .build_command(1, packet::Dialect::V2, s.id, o.value, tx)
                    .expect("builds");
                assert_eq!(pkt.len(), packet::V2_COMMAND_LEN);
                assert_eq!(pkt[19], o.wire);
            }
        }
    }
}
