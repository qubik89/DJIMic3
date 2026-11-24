//! A wireless link to a DJI Mic 3 receiver or transmitter, the way DJI Mimo
//! makes one:
//!
//! 1. Bluetooth LE: find the unit, pair (the user confirms on the unit — a
//!    short press of the transmitter's link button after it buzzes twice, or
//!    a tap on the receiver's screen), switch its Wi-Fi on and read the
//!    access point's name and passphrase, then hang up: the access point
//!    only comes up once the BLE link is gone.
//! 2. Join that access point ([`Joined`]). The unit is then reachable at
//!    192.168.0.1: TCP 60000 carries the same DUML frames as USB, and a
//!    transmitter also serves its recordings over HTTP.
//!
//! The protocol is documented in PROTOCOL.md ("Wireless file download").

mod ble;
pub mod duml;
mod net;
mod wifi;

use std::io;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;

pub use net::{Iface, TX_ADDR};

/// The TCP port of the DUML control session on a unit's access point.
pub const CONTROL_PORT: u16 = 60000;

/// Which unit to link to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Transmitter,
    Receiver,
}

impl Unit {
    /// Start of the advertised Bluetooth name.
    pub fn name_prefix(self) -> &'static str {
        match self {
            Unit::Transmitter => "DJI Mic 3 TX",
            Unit::Receiver => "DJI Mic 3 RX",
        }
    }

    /// The unit's DUML address.
    pub fn address(self) -> u8 {
        match self {
            Unit::Transmitter => duml::TX,
            Unit::Receiver => duml::RX,
        }
    }
}

/// Where a link attempt is, for a guided UI.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "step", rename_all = "kebab-case")]
pub enum Step {
    /// Waiting for the unit to advertise.
    Scanning,
    Found { name: String },
    /// Pairing requested: waiting for the user to confirm on the unit. A
    /// receiver shows `pin` on its screen as the verification code.
    Confirm { pin: String },
    /// Not confirmed in time; scanning again.
    Retry,
    Approved,
    WifiOn,
    Joining { ssid: String },
    /// On the access point, opening the session.
    Connecting,
    Listing,
    Ready,
}

#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[serde(tag = "kind", content = "detail", rename_all = "kebab-case")]
pub enum Error {
    #[error("no Bluetooth adapter")]
    NoBluetooth,
    #[error("Bluetooth is off")]
    BluetoothOff,
    #[error("Bluetooth: {0}")]
    Bluetooth(String),
    #[error("the unit refused the connection")]
    Refused,
    #[error("Wi-Fi: {0}")]
    Wifi(String),
    #[error("{0}")]
    Session(String),
    /// The computer's Wi-Fi is already joined to another unit.
    #[error("the Wi-Fi is in use by another wireless connection")]
    WifiBusy,
    #[error("cancelled")]
    Cancelled,
}

/// On the unit's access point. Call [`Joined::leave`] to give the Wi-Fi back
/// to the usual network.
pub struct Joined {
    pub ssid: String,
    /// The access point's passphrase. It's the same every time for a given
    /// unit, so callers can keep it and skip Bluetooth next time.
    pub psk: String,
    /// Joined straight away with a known passphrase (no Bluetooth).
    pub direct: bool,
    link: wifi::Link,
    left: bool,
}

impl Joined {
    pub fn iface(&self) -> &Iface {
        &self.link.iface
    }

    /// Connect to `port` on the unit (retrying while DHCP settles).
    pub fn connect(&self, port: u16, timeout: Duration) -> io::Result<TcpStream> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            match self.link.iface.connect(port, Duration::from_secs(2)) {
                Ok(s) => return Ok(s),
                Err(e) if std::time::Instant::now() >= deadline => return Err(e),
                Err(_) => std::thread::sleep(Duration::from_millis(500)),
            }
        }
    }

    /// Leave the access point (blocking: the OS tools take a moment).
    pub fn leave(mut self) {
        self.left = true;
        wifi::leave(&self.link);
    }
}

/// Never leave the computer stuck on a unit's Wi-Fi, even when a caller
/// bails out (or panics) without calling [`Joined::leave`].
impl Drop for Joined {
    fn drop(&mut self) {
        if !self.left {
            wifi::leave(&self.link);
        }
    }
}

/// A unit's access point, remembered from an earlier pairing.
#[derive(Debug, Clone)]
pub struct Known {
    pub ssid: String,
    pub psk: String,
}

/// Pair with `unit` over Bluetooth and join its Wi-Fi, retrying the
/// Bluetooth part until the user confirms on the unit or `cancel` is set.
///
/// A unit whose access point is already up (still in "connected to app"
/// mode) doesn't advertise over Bluetooth, so the `known` access points of
/// this kind of unit are tried first: one that answers is joined directly.
pub async fn pair_and_join(
    unit: Unit,
    app_id: &str,
    country: &str,
    known: &[Known],
    cancel: &AtomicBool,
    step: &(dyn Fn(Step) + Send + Sync),
) -> Result<Joined, Error> {
    for k in known.iter().filter(|k| k.ssid.starts_with(unit.name_prefix())) {
        let (ssid, psk) = (k.ssid.clone(), k.psk.clone());
        let tried = tokio::task::spawn_blocking(move || wifi::join_if_present(&ssid, &psk)).await.ok().flatten();
        if let Some(link) = tried {
            step(Step::Joining { ssid: k.ssid.clone() });
            step(Step::Connecting);
            return Ok(Joined { ssid: k.ssid.clone(), psk: k.psk.clone(), direct: true, link, left: false });
        }
        if cancel.load(Ordering::SeqCst) {
            return Err(Error::Cancelled);
        }
    }
    let adapter = ble::adapter().await?;
    let creds = loop {
        step(Step::Scanning);
        let (peripheral, name) = ble::find(&adapter, unit, cancel).await?;
        step(Step::Found { name });
        match ble::pair(&peripheral, unit, app_id, country, cancel, step).await {
            Ok(c) => break c,
            Err(ble::Attempt::Retry) => {
                if cancel.load(Ordering::SeqCst) {
                    return Err(Error::Cancelled);
                }
                step(Step::Retry);
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(ble::Attempt::Failed(e)) => return Err(e),
        }
    };
    step(Step::Joining { ssid: creds.ssid.clone() });
    let (ssid, psk) = (creds.ssid.clone(), creds.psk.clone());
    let link = tokio::task::spawn_blocking(move || wifi::join(&creds.ssid, &creds.psk, Duration::from_secs(30)))
        .await
        .map_err(|e| Error::Session(e.to_string()))??;
    step(Step::Connecting);
    Ok(Joined { ssid, psk, direct: false, link, left: false })
}

/// Clean up after a session that ended abruptly: forget any saved Wi-Fi
/// network of a DJI Mic unit. Call once at start-up.
pub fn cleanup() {
    wifi::forget_unit_networks();
}

/// A random 32-character app id, kept by the app between runs so a unit
/// sees the same "phone" every time.
pub fn new_app_id() -> String {
    (0..4).map(|_| format!("{:08x}", ble::random_u32())).collect()
}
