//! The Bluetooth half: find the unit, pair (the user confirms on it), switch
//! its Wi-Fi on and read the network's name and passphrase — then hang up so
//! the access point can start.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use btleplug::api::{Central, CentralState, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures_lite::StreamExt;
use uuid::Uuid;

use super::duml::{self, Frame, Reassembler, APP, BROADCAST, REQUEST, RESPONSE};
use super::{Error, Step, Unit};

const NOTIFY: Uuid = Uuid::from_u128(0x0000fff4_0000_1000_8000_00805f9b34fb);
const WRITE: Uuid = Uuid::from_u128(0x0000fff5_0000_1000_8000_00805f9b34fb);

/// The access point's credentials.
pub struct Wifi {
    pub ssid: String,
    pub psk: String,
}

/// How a pairing attempt ended without credentials.
pub enum Attempt {
    /// Not confirmed in time, or the transmitter hung up: try again.
    Retry,
    Failed(Error),
}

pub async fn adapter() -> Result<Adapter, Error> {
    let manager = Manager::new().await.map_err(|e| Error::Bluetooth(e.to_string()))?;
    let adapter = manager
        .adapters()
        .await
        .map_err(|e| Error::Bluetooth(e.to_string()))?
        .into_iter()
        .next()
        .ok_or(Error::NoBluetooth)?;
    // CoreBluetooth reports "unknown" for a moment after start-up.
    for _ in 0..30 {
        match adapter.adapter_state().await {
            Ok(CentralState::PoweredOn) => return Ok(adapter),
            Ok(CentralState::PoweredOff) => return Err(Error::BluetoothOff),
            _ => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
    Ok(adapter)
}

/// Wait for `unit` to advertise (a transmitter after its link button is held
/// for 2 s; a receiver whenever it's on and not linked to a phone).
pub async fn find(adapter: &Adapter, unit: Unit, cancel: &AtomicBool) -> Result<(Peripheral, String), Error> {
    adapter.start_scan(ScanFilter::default()).await.map_err(|e| Error::Bluetooth(e.to_string()))?;
    let found = 'scan: loop {
        if cancel.load(Ordering::SeqCst) {
            break Err(Error::Cancelled);
        }
        if let Ok(list) = adapter.peripherals().await {
            for p in list {
                let Ok(Some(props)) = p.properties().await else { continue };
                if let Some(name) = props.local_name.filter(|n| n.starts_with(unit.name_prefix())) {
                    break 'scan Ok((p, name));
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    };
    let _ = adapter.stop_scan().await;
    found
}

/// Pair with `p` and get its access point's credentials.
pub async fn pair(p: &Peripheral, unit: Unit, app_id: &str, country: &str, cancel: &AtomicBool, step: &(dyn Fn(Step) + Send + Sync)) -> Result<Wifi, Attempt> {
    if !p.is_connected().await.unwrap_or(false) {
        // Connecting fails when the transmitter has just stopped advertising.
        if tokio::time::timeout(Duration::from_secs(8), p.connect()).await.map_err(|_| Attempt::Retry)?.is_err() {
            return Err(Attempt::Retry);
        }
    }
    let result = session(p, unit.address(), app_id, country, cancel, step).await;
    // Hang up at once: the transmitter starts its access point only when the
    // BLE link is gone (Mimo disconnects 2 ms after reading the passphrase).
    let _ = p.disconnect().await;
    result
}

async fn session(p: &Peripheral, unit: u8, app_id: &str, country: &str, cancel: &AtomicBool, step: &(dyn Fn(Step) + Send + Sync)) -> Result<Wifi, Attempt> {
    let bt = |e: btleplug::Error| Attempt::Failed(Error::Bluetooth(e.to_string()));
    p.discover_services().await.map_err(|_| Attempt::Retry)?;
    let chars = p.characteristics();
    let find = |u: Uuid| -> Option<Characteristic> { chars.iter().find(|c| c.uuid == u).cloned() };
    let (Some(notify), Some(write)) = (find(NOTIFY), find(WRITE)) else {
        return Err(Attempt::Failed(Error::Bluetooth("this doesn't look like a DJI Mic 3".into())));
    };
    // Like Mimo: subscribe to FFF4 only.
    p.subscribe(&notify).await.map_err(|_| Attempt::Retry)?;
    let mut notes = p.notifications().await.map_err(bt)?;

    let mut seq: u16 = 0x2000 | (random_u32() as u16 & 0x0fff);
    let mut next = || {
        seq = seq.wrapping_add(1);
        seq
    };
    let write = &write;
    let send = move |frame: Vec<u8>| async move { p.write(write, &frame, WriteType::WithoutResponse).await };

    let pin = format!("{:04}", random_u32() % 10_000);
    let mut pairing = vec![app_id.len() as u8];
    pairing.extend_from_slice(app_id.as_bytes());
    pairing.push(pin.len() as u8);
    pairing.extend_from_slice(pin.as_bytes());
    pairing.extend_from_slice(&[0x01, 0x01]);
    let mut cc = *b"US";
    if country.len() == 2 && country.is_ascii() {
        cc.copy_from_slice(&country.to_ascii_uppercase().as_bytes()[..2]);
    }

    for f in [
        duml::encode(next(), APP, BROADCAST, REQUEST, 0x00, 0x2b, &[0x04, 0x00]),
        duml::encode(next(), APP, unit, REQUEST, 0x00, 0x32, &[0x11]),
        duml::encode(next(), APP, unit, REQUEST, 0x07, 0x45, &pairing),
    ] {
        send(f).await.map_err(|_| Attempt::Retry)?;
    }

    let mut frames = Reassembler::default();
    let mut heartbeat = tokio::time::interval(Duration::from_millis(2500));
    heartbeat.tick().await;
    // The transmitter gives up on an unconfirmed pairing after ~14 s.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(25);
    let (mut ssid, mut psk) = (None::<String>, None::<String>);
    let mut asked_wifi = false;
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(Attempt::Failed(Error::Cancelled));
        }
        let note = tokio::select! {
            n = notes.next() => n,
            _ = heartbeat.tick() => {
                if !p.is_connected().await.unwrap_or(false) {
                    return Err(Attempt::Retry);
                }
                let _ = send(duml::encode(next(), APP, BROADCAST, REQUEST, 0x00, 0x2b, &[0x04, 0x00])).await;
                continue;
            }
            _ = tokio::time::sleep_until(deadline) => return Err(Attempt::Retry),
        };
        let Some(note) = note else { return Err(Attempt::Retry) };
        for f in frames.push(&note.value) {
            if f.src != unit || f.dst != APP {
                continue;
            }
            match (f.set, f.id, f.is_response()) {
                // Pairing answer: 00 02 = waiting for the button, 00 01 = known.
                (0x07, 0x45, true) => match f.payload.get(1) {
                    Some(0x01) => {
                        step(Step::Approved);
                        if !asked_wifi {
                            asked_wifi = true;
                            send(duml::encode(next(), APP, unit, REQUEST, 0x07, 0x18, &[cc[0], cc[1], 0, 0])).await.map_err(|_| Attempt::Retry)?;
                        }
                    }
                    _ => step(Step::Confirm { pin: pin.clone() }),
                },
                // The user pressed the button: 01 00 = approved. Acknowledge.
                (0x07, 0x46, false) => {
                    let ack = duml::encode(f.seq, APP, f.src, RESPONSE | REQUEST, 0x07, 0x46, &[0x00]);
                    let _ = send(ack).await;
                    if f.payload.first() == Some(&0x01) {
                        step(Step::Approved);
                        if !asked_wifi {
                            asked_wifi = true;
                            send(duml::encode(next(), APP, unit, REQUEST, 0x07, 0x18, &[cc[0], cc[1], 0, 0])).await.map_err(|_| Attempt::Retry)?;
                        }
                    } else {
                        return Err(Attempt::Failed(Error::Refused));
                    }
                }
                // Wi-Fi switched on: ask for its name and passphrase.
                (0x07, 0x18, true) => {
                    step(Step::WifiOn);
                    send(duml::encode(next(), APP, unit, REQUEST, 0x07, 0x07, &[0x00])).await.map_err(|_| Attempt::Retry)?;
                    send(duml::encode(next(), APP, unit, REQUEST, 0x07, 0x0e, &[0x00])).await.map_err(|_| Attempt::Retry)?;
                }
                (0x07, 0x07, true) => ssid = Some(text(&f).ok_or(Attempt::Failed(Error::Refused))?),
                (0x07, 0x0e, true) => psk = Some(text(&f).ok_or(Attempt::Failed(Error::Refused))?),
                _ => {}
            }
            if let (Some(s), Some(k)) = (&ssid, &psk) {
                return Ok(Wifi { ssid: s.clone(), psk: k.clone() });
            }
        }
    }
}

/// `00 | len | text`; `ff …` when the transmitter refuses.
fn text(f: &Frame) -> Option<String> {
    let p = &f.payload;
    if p.first() != Some(&0) || p.len() < 2 {
        return None;
    }
    let len = p[1] as usize;
    p.get(2..2 + len).map(|b| String::from_utf8_lossy(b).into_owned())
}

/// Good enough for a PIN and a starting sequence number.
pub fn random_u32() -> u32 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut h = RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    h.finish() as u32
}
