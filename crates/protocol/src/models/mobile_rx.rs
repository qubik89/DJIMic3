//! DJI Mic Series Mobile Receiver (the USB-C plug-in receiver for phones).
//!
//! It shares the Mic Mini receiver's USB id and protocol (interface 6, v2
//! firmware) and identifies itself with the product name `DJI Mic Mini 2`.
//! Differences, from ShadowBitBasher/DJI-Mic-Control PR #4 and
//! piedol/lavdeck, both tested on hardware:
//!
//! - noise cancelling is set by the O/L/H switch on the receiver; its state
//!   is read from the transmitters' status bits and can't be written;
//! - it adds a receiver gain (−12…+12 dB in 6 dB steps, `0x0039`) and a
//!   monitoring gain (1 dB steps, `0x0026`) at status offsets 22 and 27;
//! - being bus-powered, it has no auto-off or camera power sync.
//!
//! It works with Mic Mini, Mini 2, Mini 2S, Mic 2 and Mic 3 transmitters;
//! their extra features are handled per transmitter (see `mic_mini::tx_caps`).

use super::{mic3, mic_mini, DeviceModel, MicMini};
use crate::packet;
use crate::settings::{Setting, SettingKind, V2Target};
use crate::state::DeviceStatus;

pub struct MobileRx;

const MODEL_ID: &str = "dji-mic-mobile-rx";

/// Product names the Mobile RX reports for itself.
const NAMES: [&str; 2] = ["DJI Mic Mini 2", "DJI Mic Series Mobile Receiver"];

const NC_LOCK: &str = "Use the O/L/H switch on the receiver to change noise cancelling.";

/// True once a receiver on the Mini USB id has identified itself as the
/// Mobile RX.
pub(super) fn is_mobile_receiver(status: &DeviceStatus) -> bool {
    status
        .rx
        .as_ref()
        .and_then(|rx| rx.product_name.as_deref())
        .is_some_and(|n| NAMES.contains(&n))
}

fn settings() -> &'static [Setting] {
    static CELL: std::sync::OnceLock<Vec<Setting>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
        let mut out = Vec::new();
        for s in MicMini.settings() {
            match s.id {
                "camera-power" | "auto-off-15m" => {}
                "noise-cancel" | "noise-cancel-power" | "noise-cancel-button" => {
                    let mut locked = s.clone();
                    locked.locked = Some(NC_LOCK);
                    out.push(locked);
                }
                "stereo" => {
                    out.push(s.clone());
                    out.push(rx_setting("rx-gain", "RX Gain", SettingKind::Enum, 0x0039, &mic3::RX_GAIN_OPTS));
                    out.push(rx_setting("monitor-gain", "Monitoring Gain", SettingKind::Select, 0x0026, &mic3::GAIN_1DB_OPTS));
                }
                _ => out.push(s.clone()),
            }
        }
        out
    })
}

fn rx_setting(
    id: &'static str,
    label: &'static str,
    kind: SettingKind,
    command: u16,
    options: &'static [crate::settings::SettingOption],
) -> Setting {
    Setting {
        id,
        label,
        group: "Audio",
        kind,
        v1_command: None,
        v2_command: command,
        v2_target: V2Target::Fixed(0x0000),
        options,
        exclusive_with: &[],
        note: None,
        locked: None,
    }
}

impl DeviceModel for MobileRx {
    fn id(&self) -> &'static str {
        MODEL_ID
    }
    fn name(&self) -> &'static str {
        "DJI Mic Series Mobile Receiver"
    }
    fn pictogram_key(&self) -> &'static str {
        "mobile"
    }
    fn usb_ids(&self) -> &'static [(u16, u16)] {
        MicMini.usb_ids()
    }
    fn interface(&self) -> u8 {
        MicMini.interface()
    }
    fn detach_interfaces(&self) -> &'static [u8] {
        MicMini.detach_interfaces()
    }
    fn bulk_in(&self) -> u8 {
        MicMini.bulk_in()
    }
    fn bulk_out(&self) -> u8 {
        MicMini.bulk_out()
    }
    fn settings(&self) -> &'static [Setting] {
        settings()
    }
    fn refine(&self, status: &DeviceStatus) -> Option<&'static dyn DeviceModel> {
        let named = status.rx.as_ref().and_then(|r| r.product_name.as_deref()).is_some();
        (named && !is_mobile_receiver(status)).then_some(&MicMini as &'static dyn DeviceModel)
    }

    fn decode(&self, previous: &DeviceStatus, f: &[u8]) -> Option<DeviceStatus> {
        let mut status = MicMini.decode(previous, f)?;
        status.model_id = MODEL_ID.into();
        let is_status_push = packet::heartbeat_dialect(f) == Some(packet::Dialect::V2)
            && f.get(14) == Some(&0x03)
            && f.len() >= 54
            && (f.len() - 54) % 32 == 0;
        if !is_status_push {
            return Some(status);
        }
        // Receiver gain replaces the Mini's physical dial reading.
        status.gain_dial = None;
        let s = &mut status.settings;
        s.remove("camera-power");
        s.remove("auto-off-15m");
        if let Some(v) = mic3::option_slug(&mic3::RX_GAIN_OPTS, f[22]) {
            s.insert("rx-gain".into(), v);
        }
        if let Some(v) = mic3::gain_slug(f[27]) {
            s.insert("monitor-gain".into(), v);
        }
        // Noise cancelling follows the receiver's switch and shows up in the
        // first connected transmitter's bits: +7 bit 0x01 on, +6 bit 0x20
        // strong.
        let tx_count = (f.len() - 54) / 32;
        let nc = (0..tx_count).find_map(|pos| {
            let off = 52 + pos * 32;
            mic_mini::tx_slot(f[off + 1]).map(|_| (f[off + 7] & 0x01 != 0, f[off + 6] & 0x20 != 0))
        });
        if let Some((on, strong)) = nc {
            status.nc_enabled = on;
            let s = &mut status.settings;
            s.insert("noise-cancel-power".into(), if on { "on" } else { "off" }.into());
            s.insert("noise-cancel".into(), if strong { "strong" } else { "basic" }.into());
        } else {
            let s = &mut status.settings;
            s.remove("noise-cancel-power");
            s.remove("noise-cancel");
        }
        Some(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crc;

    /// A v2 push: the 12-byte preamble, `body` from offset 12 on, and a CRC.
    fn push(body: &[u8]) -> Vec<u8> {
        let mut f = vec![0x55, 0, 0x04, 0, 0x5a, 0x02, 0x00, 0x00, 0x00, 0x5b, 0x03, 0x03];
        f.extend_from_slice(body);
        f[1] = (f.len() + 2) as u8;
        f[3] = packet::header_crc8(&f[..3]);
        crc::append(f, crc::V2_INIT, crc::V2_RESIDUE)
    }

    /// Identity push with one product-name record for `unit`.
    fn name_record(unit: u8, name: &str) -> Vec<u8> {
        let mut body = vec![0, 0, 0x06, unit, 0, 0, 0, name.len() as u8];
        body.extend_from_slice(name.as_bytes());
        let len = (body.len() - 2) as u16;
        body[0] = len as u8;
        body[1] = (len >> 8) as u8;
        push(&body)
    }

    /// Status push with one transmitter (TX1) whose slot data from +6 is `slot`.
    fn status_1tx(header: &[(usize, u8)], slot: &[(usize, u8)]) -> Vec<u8> {
        let mut body = vec![0u8; 52 - 12 + 32];
        body[0] = 0x46;
        body[2] = 0x03;
        body[44 - 12] = 0x01; // TX1 connected
        let s = 52 - 12;
        body[s] = 0x02;
        body[s + 1] = 0x01;
        body[s + 5] = 0x1a;
        for &(off, v) in header {
            body[off - 12] = v;
        }
        for &(off, v) in slot {
            body[s + off] = v;
        }
        push(&body)
    }

    fn named(status: DeviceStatus, rx: Option<&str>, tx1: Option<&str>) -> DeviceStatus {
        let mut st = status;
        if let Some(n) = rx {
            st = MicMini.decode(&st, &name_record(0, n)).unwrap();
        }
        if let Some(n) = tx1 {
            st = MicMini.decode(&st, &name_record(1, n)).unwrap();
        }
        st
    }

    #[test]
    fn mini_receiver_switches_to_mobile_rx_by_name() {
        let st = named(DeviceStatus::disconnected("dji-mic-mini"), Some("DJI Mic Mini 2"), None);
        assert_eq!(MicMini.refine(&st).map(|m| m.id()), Some(MODEL_ID));
        assert!(MobileRx.refine(&st).is_none());
        let mini = named(DeviceStatus::disconnected("dji-mic-mini"), Some("DJI Mic Mini"), None);
        assert!(MicMini.refine(&mini).is_none());
        assert_eq!(MobileRx.refine(&mini).map(|m| m.id()), Some("dji-mic-mini"));
    }

    #[test]
    fn mobile_rx_reads_gains_and_switch_noise_cancelling() {
        let prev = named(DeviceStatus::disconnected("dji-mic-mini"), Some("DJI Mic Mini 2"), None);
        let f = status_1tx(&[(20, 0x80), (22, 0xfa), (27, 0xfd)], &[(6, 0x20), (7, 0x01)]);
        let st = MobileRx.decode(&prev, &f).unwrap();
        assert_eq!(st.model_id, MODEL_ID);
        assert_eq!(st.settings["rx-gain"], "m6");
        assert_eq!(st.settings["monitor-gain"], "m3");
        assert_eq!(st.settings["noise-cancel-power"], "on");
        assert_eq!(st.settings["noise-cancel"], "strong");
        assert!(st.nc_enabled);
        assert!(!st.settings.contains_key("camera-power"));
        assert!(!st.settings.contains_key("auto-off-15m"));
        assert_eq!(st.gain_dial, None);
        // Writes to the switch-controlled settings are refused.
        let nc = MobileRx.settings().iter().find(|s| s.id == "noise-cancel-power").unwrap();
        assert!(nc.locked.is_some());
        assert!(MobileRx.settings().iter().all(|s| s.id != "camera-power"));
    }

    #[test]
    fn mic2_transmitter_extended_fields_on_a_mini_receiver() {
        // Slot bytes from nicolasmmb/DJI-Mic-Utility's Mic 2 capture:
        // +9=28 (32-bit float, low cut), +11=58, +13=fd, +14/+16 = 11.2 h.
        let prev = named(DeviceStatus::disconnected("dji-mic-mini"), Some("DJI Mic Mini 2"), Some("DJI Mic 2"));
        let f = status_1tx(&[], &[(6, 0x04), (9, 0x28), (11, 0x58), (13, 0xfd), (14, 0x70), (16, 0x70)]);
        let st = MicMini.decode(&prev, &f).unwrap();
        let tx = st.tx[0].as_ref().unwrap();
        assert_eq!(tx.settings["mute"], "on");
        assert_eq!(tx.settings["record"], "off");
        assert_eq!(tx.rec_total, Some(112));
        assert_eq!(tx.rec_remaining, Some(112));
        assert!(!tx.settings.contains_key("voice-tone"), "not validated on Mic 2");
        assert!(!tx.settings.contains_key("tx-gain"), "not validated on Mic 2");
        assert_eq!(st.settings["float32"], "on");
        assert_eq!(st.settings["rec-stop"], "off");
        assert!(!st.settings.contains_key("loop-recording"));
    }

    #[test]
    fn mini_transmitters_keep_the_basic_feature_set() {
        let prev = named(DeviceStatus::disconnected("dji-mic-mini"), Some("DJI Mic Mini"), Some("DJI Mic Mini"));
        let st = MicMini.decode(&prev, &status_1tx(&[], &[(9, 0x41)])).unwrap();
        let tx = st.tx[0].as_ref().unwrap();
        assert!(tx.settings.is_empty());
        assert!(!st.settings.contains_key("float32"));

        let prev = named(DeviceStatus::disconnected("dji-mic-mini"), None, Some("DJI Mic Mini 2"));
        let st = MicMini.decode(&prev, &status_1tx(&[], &[(9, 0x41)])).unwrap();
        assert_eq!(st.tx[0].as_ref().unwrap().settings["voice-tone"], "rich");
    }
}
