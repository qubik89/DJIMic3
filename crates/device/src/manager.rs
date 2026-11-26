//! Discovery and orchestration of all connected devices.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

use nusb::MaybeFuture;
use protocol::{model_for_usb, DeviceModel, DeviceStatus, Setting, V2Target};
use serde::Serialize;

use crate::actor::{self, now_millis, Actor, Transport};
use crate::diag;
use crate::error::{DeviceError, Result};
use crate::platform::{current_os, Os};

/// A device is "connected" only if a heartbeat arrived within this window;
/// otherwise it is present on the bus but not streaming (e.g. powered off).
/// Real heartbeat timing is bursty enough that anything shorter flickers.
const LIVE_WINDOW_MS: u64 = 1500;

/// One tracked, opened device.
struct Handle {
    actor: Actor,
}

impl Handle {
    /// The model currently driving the device (it can change once the
    /// receiver identifies itself).
    fn model(&self) -> &'static dyn DeviceModel {
        *self.actor.model.lock().unwrap()
    }

    /// True if a heartbeat was decoded within [`LIVE_WINDOW_MS`].
    fn is_live(&self) -> bool {
        let last = self.actor.last_seen.load(Ordering::Relaxed);
        last != 0 && now_millis().saturating_sub(last) <= LIVE_WINDOW_MS
    }
}

/// Just enough per-transmitter state for a device-list row's battery
/// indicator — a trimmed-down [`protocol::TxInfo`] rather than the whole
/// thing, since that's all list/picker UI needs without fetching each
/// device's full status.
#[derive(Debug, Clone, Serialize)]
pub struct TxBattery {
    pub battery: Option<u8>,
    pub charging: bool,
}

/// A lightweight description of a device for lists and pickers.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceSummary {
    pub id: String,
    pub model_id: String,
    pub model_name: String,
    pub pictogram_key: String,
    pub connected: bool,
    pub rx_serial: Option<String>,
    /// Battery/charging state for each TX slot, `None` where no transmitter
    /// is connected there.
    pub tx: [Option<TxBattery>; protocol::MAX_TX],
    /// How many transmitter slots this model has (see `DeviceModel::max_tx`).
    pub max_tx: usize,
}

/// The result of scanning the bus, used to decide whether to show setup help.
#[derive(Debug, Clone, Serialize)]
pub struct Probe {
    pub os: Os,
    /// Matching devices seen on the bus.
    pub present: usize,
    /// Matching devices we could open.
    pub accessible: usize,
    /// A device was present but could not be opened (likely udev on Linux).
    pub permission_issue: bool,
}

/// Owns every device task and mediates access from the front-ends.
pub struct DeviceManager {
    handles: Mutex<HashMap<String, Handle>>,
    probe: Mutex<Probe>,
    /// Why a matching device couldn't be opened, by bus position, for the
    /// diagnostic report. Cleared once it opens.
    open_errors: Mutex<HashMap<String, String>>,
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            handles: Mutex::new(HashMap::new()),
            probe: Mutex::new(Probe {
                os: current_os(),
                present: 0,
                accessible: 0,
                permission_issue: false,
            }),
            open_errors: Mutex::new(HashMap::new()),
        }
    }

    /// Enumerate the bus, dropping vanished devices and adopting new ones.
    ///
    /// Cheap and idempotent — call it periodically (the GUI) or once before an
    /// action (the CLI).
    pub fn refresh(&self) {
        let mut handles = self.handles.lock().unwrap();
        handles.retain(|_, h| h.actor.alive.load(Ordering::Relaxed));

        let mut present = 0;
        let mut permission_issue = false;

        // On Android, apps can't enumerate USB devices themselves: the app
        // gets each receiver from the system and hands it over with
        // `adopt_fd`, so there's nothing to scan here.
        #[cfg(not(target_os = "android"))]
        {
            let devices = match nusb::list_devices().wait() {
                Ok(d) => d,
                Err(_) => return,
            };

            for info in devices {
                let Some(model) = model_for_usb(info.vendor_id(), info.product_id()) else {
                    continue;
                };
                present += 1;
                let id = device_id(info.serial_number(), info.bus_id(), info.device_address());
                if handles.contains_key(&id) {
                    continue;
                }
                let place = bus_place(info.bus_id(), info.device_address());
                match info.open().wait().and_then(|dev| claim(model, &dev)) {
                    Ok(iface) => {
                        let actor = actor::spawn(model, Transport::Usb(iface));
                        handles.insert(id, Handle { actor });
                        self.open_errors.lock().unwrap().remove(&place);
                    }
                    Err(e) => {
                        if e.kind() == nusb::ErrorKind::PermissionDenied {
                            permission_issue = true;
                        }
                        let why = format!("{:?}: {e}", e.kind());
                        self.open_errors.lock().unwrap().insert(place, why);
                    }
                }
            }
        }
        #[cfg(target_os = "android")]
        {
            present = handles.len();
        }

        *self.probe.lock().unwrap() = Probe {
            os: current_os(),
            present,
            accessible: handles.len(),
            permission_issue,
        };
    }

    /// Take over a receiver the platform has already opened (Android hands
    /// apps a file descriptor for a USB device once the user allows it).
    /// Returns the device id, or the existing one if it's already tracked.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn adopt_fd(
        &self,
        fd: std::os::fd::OwnedFd,
        vendor: u16,
        product: u16,
        serial: Option<&str>,
    ) -> Result<String> {
        let model = model_for_usb(vendor, product)
            .ok_or_else(|| DeviceError::NotFound(format!("{vendor:04x}:{product:04x}")))?;
        let mut handles = self.handles.lock().unwrap();
        handles.retain(|_, h| h.actor.alive.load(Ordering::Relaxed));
        let id = device_id(serial, "fd", 0);
        if handles.contains_key(&id) {
            return Ok(id);
        }
        let dev = nusb::Device::from_fd(fd).wait().map_err(|e| DeviceError::Usb(e.to_string()))?;
        let iface = claim(model, &dev).map_err(|e| DeviceError::Usb(e.to_string()))?;
        handles.insert(id.clone(), Handle { actor: actor::spawn(model, Transport::Usb(iface)) });
        let mut probe = self.probe.lock().unwrap();
        probe.present = handles.len();
        probe.accessible = handles.len();
        Ok(id)
    }

    /// Take over a Mic 3 receiver reached over its own Wi-Fi: `stream` is the
    /// DUML session on its access point (TCP 60000), `label` its network
    /// name. Returns the device id.
    pub fn adopt_stream(&self, label: &str, stream: std::net::TcpStream) -> Result<String> {
        // The Mic 3 receiver's USB id picks its model; the receiver confirms
        // it once it answers "get version".
        let model = model_for_usb(0x2ca3, 0x4015).ok_or_else(|| DeviceError::NotFound("DJI Mic 3".into()))?;
        let id = format!("wifi:{label}");
        let mut handles = self.handles.lock().unwrap();
        handles.insert(id.clone(), Handle { actor: actor::spawn(model, Transport::Tcp(stream)) });
        Ok(id)
    }

    /// Stop tracking a device (closes a Wi-Fi receiver's session). Returns
    /// whether it was tracked.
    pub fn forget(&self, id: &str) -> bool {
        self.handles.lock().unwrap().remove(id).is_some()
    }

    /// Whether `id` is tracked and its link still open.
    pub fn is_alive(&self, id: &str) -> bool {
        self.handles.lock().unwrap().get(id).is_some_and(|h| h.actor.alive.load(Ordering::Relaxed))
    }

    /// The most recent bus-scan result.
    pub fn probe(&self) -> Probe {
        self.probe.lock().unwrap().clone()
    }

    /// Summaries of all currently tracked devices.
    pub fn list(&self) -> Vec<DeviceSummary> {
        let handles = self.handles.lock().unwrap();
        let mut out: Vec<DeviceSummary> = handles
            .iter()
            .map(|(id, h)| {
                let live = h.is_live();
                let st = h.actor.status.lock().unwrap();
                DeviceSummary {
                    id: id.clone(),
                    model_id: h.model().id().to_string(),
                    model_name: h.model().name().to_string(),
                    pictogram_key: h.model().pictogram_key().to_string(),
                    connected: live,
                    rx_serial: st.rx.as_ref().and_then(|r| r.serial.clone()),
                    max_tx: h.model().max_tx(),
                    tx: std::array::from_fn(|i| {
                        st.tx[i].as_ref().map(|t| TxBattery {
                            battery: t.battery,
                            charging: t.charging.unwrap_or(false),
                        })
                    }),
                }
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// The id of the sole tracked device, if exactly one is present.
    pub fn only_device(&self) -> Option<String> {
        let handles = self.handles.lock().unwrap();
        if handles.len() == 1 {
            handles.keys().next().cloned()
        } else {
            None
        }
    }

    /// A clone of the latest status for `id`.
    pub fn status(&self, id: &str) -> Result<DeviceStatus> {
        let handles = self.handles.lock().unwrap();
        let h = handles.get(id).ok_or_else(|| DeviceError::NotFound(id.to_string()))?;
        let live = h.is_live();
        let mut status = h.actor.status.lock().unwrap().clone();
        // Reflect live streaming, not merely "has ever connected".
        status.connected = live;
        Ok(status)
    }

    /// The settings descriptors for `id`'s model.
    pub fn settings(&self, id: &str) -> Result<&'static [Setting]> {
        let handles = self.handles.lock().unwrap();
        let h = handles.get(id).ok_or_else(|| DeviceError::NotFound(id.to_string()))?;
        Ok(h.model().settings())
    }

    /// Apply `value` to `setting` on device `id`, blocking until acknowledged.
    ///
    /// For a setting addressed to one specific transmitter (its
    /// [`V2Target`] is `Tx`, e.g. Voice Tone), use
    /// [`set_tx`](Self::set_tx) instead.
    pub fn set(&self, id: &str, setting: &str, value: &str) -> Result<()> {
        self.send_set(id, setting, value, None)
    }

    /// Apply `value` to `setting` on transmitter slot `tx` (0-based) of
    /// device `id`, blocking until acknowledged. Only valid for a setting
    /// addressed to one specific transmitter (its [`V2Target`] is `Tx`, e.g.
    /// Voice Tone) — every other setting mirrors across TX or targets the
    /// receiver and should use [`set`](Self::set).
    pub fn set_tx(&self, id: &str, tx: usize, setting: &str, value: &str) -> Result<()> {
        self.send_set(id, setting, value, Some(tx))
    }

    fn send_set(&self, id: &str, setting: &str, value: &str, tx: Option<usize>) -> Result<()> {
        let cmd_tx = {
            let handles = self.handles.lock().unwrap();
            let h = handles.get(id).ok_or_else(|| DeviceError::NotFound(id.to_string()))?;

            // Validate up-front for clear errors without a round-trip.
            let desc = h
                .model()
                .settings()
                .iter()
                .find(|s| s.id == setting)
                .ok_or_else(|| DeviceError::UnknownSetting(setting.to_string()))?;
            if let Some(reason) = desc.locked {
                return Err(DeviceError::Locked(reason.to_string()));
            }
            if desc.option(value).is_none() {
                return Err(DeviceError::UnknownValue {
                    setting: setting.to_string(),
                    value: value.to_string(),
                });
            }
            match (desc.v2_target, tx) {
                (V2Target::Tx, None) => return Err(DeviceError::RequiresTx(setting.to_string())),
                (V2Target::Fixed(_), Some(_)) => {
                    return Err(DeviceError::UnexpectedTx(setting.to_string()))
                }
                _ => {}
            }
            h.actor.cmd_tx.clone()
        };

        let (resp_tx, resp_rx) = std::sync::mpsc::channel();
        cmd_tx
            .send_blocking(actor::Command {
                setting_id: setting.to_string(),
                value: value.to_string(),
                tx,
                resp: resp_tx,
            })
            .map_err(|_| DeviceError::Disconnected)?;
        resp_rx.recv().map_err(|_| DeviceError::Disconnected)?
    }
}

/// Claim the model's control interface (detaching a kernel driver if one
/// holds it) and select its alternate setting.
fn claim(model: &'static dyn DeviceModel, dev: &nusb::Device) -> std::result::Result<nusb::Interface, nusb::Error> {
    for &iface in model.detach_interfaces() {
        let _ = dev.detach_kernel_driver(iface);
    }
    let iface = match dev.claim_interface(model.interface()).wait() {
        Ok(i) => i,
        Err(_) => dev.detach_and_claim_interface(model.interface()).wait()?,
    };
    if model.alt_setting() != 0 {
        iface.set_alt_setting(model.alt_setting()).wait()?;
    }
    Ok(iface)
}

/// Where a device sits on the bus, for messages that outlive one scan.
#[cfg_attr(target_os = "android", allow(dead_code))]
fn bus_place(bus: &str, address: u8) -> String {
    format!("bus {bus} address {address}")
}

/// A stable per-session device id: the USB serial if present, else bus/address.
fn device_id(serial: Option<&str>, bus: &str, address: u8) -> String {
    match serial {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => format!("usb-{bus}-{address}"),
    }
}

/// DJI's USB vendor id.
#[cfg_attr(target_os = "android", allow(dead_code))]
const DJI_VENDOR: u16 = 0x2ca3;

/// A readable name for a USB class code.
#[cfg_attr(target_os = "android", allow(dead_code))]
fn class_name(class: u8) -> &'static str {
    match class {
        0x00 => "per interface",
        0x01 => "audio",
        0x02 => "communications",
        0x03 => "HID",
        0x06 => "still image / MTP",
        0x08 => "mass storage",
        0x0a => "CDC data",
        0x0e => "video",
        0xef => "miscellaneous",
        0xfe => "application specific",
        0xff => "vendor specific",
        _ => "other",
    }
}

/// Replace every `"serial"` string in a JSON value with its last four
/// characters.
fn redact_serials(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, v) in map.iter_mut() {
                match v {
                    serde_json::Value::String(s) if key == "serial" => *s = diag::redact(s),
                    _ => redact_serials(v),
                }
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(redact_serials),
        _ => {}
    }
}

impl DeviceManager {
    /// A plain-text troubleshooting report to paste into an issue or an
    /// email: every DJI device on the bus with its interfaces and whether it
    /// could be opened, then for each open device its decoded status, the
    /// commands sent and how they were answered, and raw frames. Serial
    /// numbers are cut to their last four characters, in raw bytes too.
    pub fn diagnostics(&self, app_version: &str) -> String {
        use std::fmt::Write as _;

        self.refresh();
        let probe = self.probe();
        let handles = self.handles.lock().unwrap();
        let open_errors = self.open_errors.lock().unwrap().clone();

        // Serial numbers to mask: what the devices reported, and USB serials.
        let mut hide: Vec<String> = Vec::new();
        for h in handles.values() {
            let st = h.actor.status.lock().unwrap();
            hide.extend(st.rx.as_ref().and_then(|r| r.serial.clone()));
            hide.extend(st.tx.iter().flatten().filter_map(|t| t.serial.clone()));
        }

        let mut out = String::new();
        let _ = writeln!(out, "DJI Mic Control diagnostic report");
        let _ = writeln!(
            out,
            "app {app_version} · {} {} · {}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            diag::utc(now_millis())
        );
        let _ = writeln!(
            out,
            "supported receivers on the bus: {} · opened: {}{}",
            probe.present,
            probe.accessible,
            if probe.permission_issue { " · permission denied" } else { "" }
        );

        let _ = writeln!(out, "\n== DJI USB devices ==");
        #[cfg(target_os = "android")]
        let _ = writeln!(out, "Not listed on Android: the system hands receivers to the app one by one.");
        #[cfg(not(target_os = "android"))]
        match nusb::list_devices().wait() {
            Ok(devices) => {
                let mut found = 0;
                for info in devices {
                    let named_dji = [info.manufacturer_string(), info.product_string()]
                        .iter()
                        .flatten()
                        .any(|s| s.to_ascii_lowercase().contains("dji"));
                    if info.vendor_id() != DJI_VENDOR && !named_dji {
                        continue;
                    }
                    found += 1;
                    hide.extend(info.serial_number().map(str::to_string));
                    let place = bus_place(info.bus_id(), info.device_address());
                    let _ = writeln!(
                        out,
                        "{:04x}:{:04x} \"{}\" by \"{}\" · {} · {}",
                        info.vendor_id(),
                        info.product_id(),
                        info.product_string().unwrap_or("?"),
                        info.manufacturer_string().unwrap_or("?"),
                        info.speed().map(|s| format!("{s:?}").to_lowercase() + " speed").unwrap_or_else(|| "speed unknown".into()),
                        place
                    );
                    let _ = writeln!(
                        out,
                        "  device class {:02x}/{:02x}/{:02x} ({})",
                        info.class(),
                        info.subclass(),
                        info.protocol(),
                        class_name(info.class())
                    );
                    for i in info.interfaces() {
                        let _ = writeln!(
                            out,
                            "  interface {}: {:02x}/{:02x}/{:02x} ({}){}",
                            i.interface_number(),
                            i.class(),
                            i.subclass(),
                            i.protocol(),
                            class_name(i.class()),
                            i.interface_string().map(|s| format!(" \"{s}\"")).unwrap_or_default()
                        );
                    }
                    let state = match model_for_usb(info.vendor_id(), info.product_id()) {
                        None => "not a receiver this version of the app knows".to_string(),
                        Some(model) => {
                            let id = device_id(info.serial_number(), info.bus_id(), info.device_address());
                            let state = if handles.contains_key(&id) {
                                "opened".to_string()
                            } else if let Some(why) = open_errors.get(&place) {
                                format!("could not be opened: {why}")
                            } else {
                                "not opened".to_string()
                            };
                            format!("{} (interface {}) · {state}", model.name(), model.interface())
                        }
                    };
                    let _ = writeln!(out, "  app: {state}");
                }
                if found == 0 {
                    let _ = writeln!(out, "None found. Check the cable (some only charge) and that the receiver is on.");
                }
            }
            Err(e) => {
                let _ = writeln!(out, "USB scan failed: {e}");
            }
        }

        let _ = writeln!(out, "\n== Devices the app opened ==");
        if handles.is_empty() {
            let _ = writeln!(out, "None.");
        }
        let mut ids: Vec<&String> = handles.keys().collect();
        ids.sort();
        for id in ids {
            let h = &handles[id];
            let shown = if id.starts_with("usb-") || id.starts_with("wifi:") { id.clone() } else { diag::redact(id) };
            let last = h.actor.last_seen.load(Ordering::Relaxed);
            let _ = writeln!(
                out,
                "{shown} · {} ({}) · {} · {}",
                h.model().name(),
                h.model().id(),
                if h.actor.alive.load(Ordering::Relaxed) { "link open" } else { "link closed" },
                if last == 0 {
                    "no status decoded yet".to_string()
                } else {
                    format!("last status {} ago", diag::secs(now_millis().saturating_sub(last)))
                }
            );
            h.actor.trace.lock().unwrap().render(&mut out, &hide);
            let status = h.actor.status.lock().unwrap().clone();
            if let Ok(mut json) = serde_json::to_value(&status) {
                redact_serials(&mut json);
                let text = serde_json::to_string_pretty(&json).unwrap_or_default();
                let _ = writeln!(out, "  decoded status:");
                for line in text.lines() {
                    let _ = writeln!(out, "    {line}");
                }
            }
        }
        out
    }
}
