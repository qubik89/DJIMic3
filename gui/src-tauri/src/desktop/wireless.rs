//! Wireless links: downloading recordings over a transmitter's own Wi-Fi,
//! and controlling a receiver over its Wi-Fi like a USB one.
//!
//! The webview drives a guided flow: [`wireless_connect`] runs Bluetooth
//! pairing (the user confirms on the transmitter), joins the access point and
//! lists the takes, reporting each stage as a `wireless-step` event. Once
//! connected, the transmitter shows up in the recordings list like a USB one
//! ([`wireless_transmitter`]) and imports go through [`wireless_import`],
//! which reports with the same `recordings-progress` / `recordings-done`
//! events as USB imports.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use std::collections::HashMap;

use device::DeviceManager;
use link::{Joined, Unit};
use recordings::wireless::{self, Connection, Error};
use recordings::{Outcome, Progress, Transmitter};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
pub struct WirelessState {
    conn: Mutex<Option<Arc<Connection>>>,
    /// Receivers controlled over Wi-Fi, by device id.
    receivers: Mutex<HashMap<String, Joined>>,
    connecting: AtomicBool,
    cancel: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
    job_cancel: Arc<AtomicBool>,
}

impl WirelessState {
    fn current(&self) -> Option<Arc<Connection>> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The computer has one Wi-Fi radio: one wireless link at a time.
    fn wifi_in_use(&self) -> bool {
        self.current().is_some() || !self.receivers.lock().unwrap_or_else(|e| e.into_inner()).is_empty()
    }
}

/// The id this app pairs with, kept between runs so the transmitter sees
/// the same "phone" every time.
fn app_id(app: &AppHandle) -> String {
    let file = app.path().app_data_dir().ok().map(|d| d.join("wireless-app-id"));
    if let Some(id) = file.as_ref().and_then(|f| std::fs::read_to_string(f).ok()) {
        let id = id.trim().to_string();
        if id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return id;
        }
    }
    let id = wireless::new_app_id();
    if let Some(f) = file {
        let _ = f.parent().map(std::fs::create_dir_all);
        let _ = std::fs::write(f, &id);
    }
    id
}

/// Access points of units paired before (`wireless-networks`: one
/// `ssid<TAB>passphrase` line each). A unit's passphrase never changes, and
/// knowing it lets the app rejoin a unit whose Wi-Fi is still up.
fn known_networks(app: &AppHandle) -> Vec<link::Known> {
    let Some(file) = app.path().app_data_dir().ok().map(|d| d.join("wireless-networks")) else { return Vec::new() };
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(s, p)| link::Known { ssid: s.to_string(), psk: p.to_string() })
        .collect()
}

fn remember_network(app: &AppHandle, ssid: &str, psk: &str) {
    let Some(file) = app.path().app_data_dir().ok().map(|d| d.join("wireless-networks")) else { return };
    let mut all: Vec<link::Known> = known_networks(app).into_iter().filter(|k| k.ssid != ssid).collect();
    all.push(link::Known { ssid: ssid.to_string(), psk: psk.to_string() });
    let text: String = all.iter().map(|k| format!("{}\t{}\n", k.ssid, k.psk)).collect();
    let _ = file.parent().map(std::fs::create_dir_all);
    let _ = std::fs::write(file, text);
}

/// Pair with a transmitter and join its Wi-Fi. `country` is the two-letter
/// region the transmitter's Wi-Fi should follow (from the system locale).
#[tauri::command]
pub async fn wireless_connect(app: AppHandle, country: String) -> Result<(), Error> {
    let state = app.state::<WirelessState>();
    if state.current().is_some() {
        return Ok(());
    }
    if state.wifi_in_use() {
        return Err(Error::WifiBusy);
    }
    if state.connecting.swap(true, Ordering::SeqCst) {
        return Err(Error::Session("already connecting".into()));
    }
    state.cancel.store(false, Ordering::SeqCst);
    let cancel = state.cancel.clone();
    let id = app_id(&app);
    let emitter = app.clone();
    let step = move |s: wireless::Step| {
        let _ = emitter.emit("wireless-step", &s);
    };
    let known = known_networks(&app);
    let result = wireless::connect(&id, &country, &known, &cancel, &step).await;
    state.connecting.store(false, Ordering::SeqCst);
    let conn = result?;
    let k = conn.known();
    remember_network(&app, &k.ssid, &k.psk);
    *state.conn.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(conn));
    Ok(())
}

/// Stop a connection attempt.
#[tauri::command]
pub fn wireless_cancel(state: State<'_, WirelessState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

/// The connected transmitter, with `imported` set against `dest`. When the
/// link has dropped, the connection is closed and `wireless-lost` emitted.
#[tauri::command]
pub fn wireless_transmitter(app: AppHandle, dest: Option<String>, state: State<'_, WirelessState>) -> Option<Transmitter> {
    let conn = state.current()?;
    if !conn.is_alive() && !state.busy.load(Ordering::SeqCst) {
        let gone = state.conn.lock().unwrap_or_else(|e| e.into_inner()).take();
        drop(conn);
        if let Some(c) = gone {
            std::thread::spawn(move || {
                if let Ok(c) = Arc::try_unwrap(c) {
                    c.close();
                }
            });
        }
        let _ = app.emit("wireless-lost", ());
        return None;
    }
    let mut txs = vec![conn.transmitter().clone()];
    if let Some(dest) = dest.filter(|d| !d.is_empty()) {
        recordings::mark_imported(&mut txs, Path::new(&dest));
    }
    txs.pop()
}

/// Download `files` (server paths from [`wireless_transmitter`]) into `dest`.
#[tauri::command]
pub fn wireless_import(app: AppHandle, files: Vec<String>, dest: String, state: State<'_, WirelessState>) -> Result<(), String> {
    if dest.trim().is_empty() {
        return Err("No import folder".into());
    }
    let conn = state.current().ok_or("The transmitter is not connected")?;
    let files: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
    let jobs = recordings::plan(std::slice::from_ref(conn.transmitter()), &files, Path::new(&dest))?;
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("An import is already running".into());
    }
    state.job_cancel.store(false, Ordering::SeqCst);
    let (busy, cancel) = (state.busy.clone(), state.job_cancel.clone());
    std::thread::spawn(move || {
        let mut last = Instant::now() - Duration::from_secs(1);
        let outcome: Outcome = conn.download(&jobs, &cancel, |p: &Progress| {
            if p.done_bytes == p.total_bytes || last.elapsed() >= Duration::from_millis(100) {
                last = Instant::now();
                let _ = app.emit("recordings-progress", p);
            }
        });
        busy.store(false, Ordering::SeqCst);
        let _ = app.emit("recordings-done", &outcome);
    });
    Ok(())
}

/// Stop the running download after the current chunk.
#[tauri::command]
pub fn wireless_import_cancel(state: State<'_, WirelessState>) {
    state.job_cancel.store(true, Ordering::SeqCst);
}

/// Close the session and put the Wi-Fi back on the usual network.
#[tauri::command]
pub async fn wireless_disconnect(app: AppHandle) -> Result<(), String> {
    let state = app.state::<WirelessState>();
    if state.busy.load(Ordering::SeqCst) {
        return Err("Wait for the download to finish".into());
    }
    let Some(conn) = state.conn.lock().unwrap_or_else(|e| e.into_inner()).take() else {
        return Ok(());
    };
    tauri::async_runtime::spawn_blocking(move || {
        // The status poll may hold a clone for a moment.
        let mut conn = conn;
        for _ in 0..50 {
            match Arc::try_unwrap(conn) {
                Ok(c) => return c.close(),
                Err(c) => {
                    conn = c;
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Pair with a Mic 3 receiver over Bluetooth, join its Wi-Fi and control it
/// like a USB one. Returns the new device id. Steps are reported as
/// `wireless-rx-step` events.
#[tauri::command]
pub async fn rx_wireless_connect(app: AppHandle, country: String) -> Result<String, Error> {
    let state = app.state::<WirelessState>();
    if state.wifi_in_use() {
        return Err(Error::WifiBusy);
    }
    if state.connecting.swap(true, Ordering::SeqCst) {
        return Err(Error::Session("already connecting".into()));
    }
    state.cancel.store(false, Ordering::SeqCst);
    let cancel = state.cancel.clone();
    let id = app_id(&app);
    let emitter = app.clone();
    let step = move |s: link::Step| {
        let _ = emitter.emit("wireless-rx-step", &s);
    };
    let known = known_networks(&app);
    let joined = link::pair_and_join(Unit::Receiver, &id, &country, &known, &cancel, &step).await;
    state.connecting.store(false, Ordering::SeqCst);
    let joined = joined?;
    remember_network(&app, &joined.ssid, &joined.psk);
    let stream = match tauri::async_runtime::spawn_blocking({
        let iface = joined.iface().clone();
        move || -> std::io::Result<std::net::TcpStream> {
            let deadline = Instant::now() + Duration::from_secs(20);
            loop {
                match iface.connect(link::CONTROL_PORT, Duration::from_secs(2)) {
                    Ok(s) => return Ok(s),
                    Err(e) if Instant::now() >= deadline => return Err(e),
                    Err(_) => std::thread::sleep(Duration::from_millis(500)),
                }
            }
        }
    })
    .await
    {
        Ok(Ok(s)) => s,
        other => {
            let msg = match other {
                Ok(Err(e)) => e.to_string(),
                Err(e) => e.to_string(),
                Ok(Ok(_)) => unreachable!(),
            };
            tauri::async_runtime::spawn_blocking(move || joined.leave());
            return Err(Error::Session(format!("control channel: {msg}")));
        }
    };
    let mgr = app.state::<Arc<DeviceManager>>().inner().clone();
    let device = match mgr.adopt_stream(&joined.ssid, stream) {
        Ok(d) => d,
        Err(e) => {
            tauri::async_runtime::spawn_blocking(move || joined.leave());
            return Err(Error::Session(e.to_string()));
        }
    };
    state.receivers.lock().unwrap_or_else(|e| e.into_inner()).insert(device.clone(), joined);
    let _ = app.emit("wireless-rx-step", &link::Step::Ready);

    // When the link drops (receiver off, out of range), give the Wi-Fi back.
    let watch = app.clone();
    let watched = device.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(2));
        let state = watch.state::<WirelessState>();
        let still_ours = state.receivers.lock().unwrap_or_else(|e| e.into_inner()).contains_key(&watched);
        if !still_ours {
            return;
        }
        if !mgr.is_alive(&watched) {
            mgr.forget(&watched);
            if let Some(j) = state.receivers.lock().unwrap_or_else(|e| e.into_inner()).remove(&watched) {
                j.leave();
            }
            let _ = watch.emit("wireless-rx-lost", &watched);
            return;
        }
    });
    Ok(device)
}

/// Stop a receiver connection attempt.
#[tauri::command]
pub fn rx_wireless_cancel(state: State<'_, WirelessState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

/// Close a Wi-Fi receiver's session and put the Wi-Fi back on the usual network.
#[tauri::command]
pub async fn rx_wireless_disconnect(app: AppHandle, device: String) -> Result<(), String> {
    let mgr = app.state::<Arc<DeviceManager>>().inner().clone();
    mgr.forget(&device);
    let joined = app.state::<WirelessState>().receivers.lock().unwrap_or_else(|e| e.into_inner()).remove(&device);
    if let Some(j) = joined {
        tauri::async_runtime::spawn_blocking(move || j.leave()).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Device ids of the receivers connected over Wi-Fi.
#[tauri::command]
pub fn rx_wireless_devices(state: State<'_, WirelessState>) -> Vec<String> {
    state.receivers.lock().unwrap_or_else(|e| e.into_inner()).keys().cloned().collect()
}

/// Close every wireless link before the app exits (blocking).
pub fn shutdown(app: &AppHandle) {
    let state = app.state::<WirelessState>();
    state.cancel.store(true, Ordering::SeqCst);
    state.job_cancel.store(true, Ordering::SeqCst);
    let mgr = app.state::<Arc<DeviceManager>>().inner().clone();
    let receivers: Vec<(String, Joined)> = state.receivers.lock().unwrap_or_else(|e| e.into_inner()).drain().collect();
    for (id, joined) in receivers {
        mgr.forget(&id);
        joined.leave();
    }
    let conn = state.conn.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(conn) = conn {
        match Arc::try_unwrap(conn) {
            Ok(c) => c.close(),
            // An import thread still holds it: dropping our handle is all we
            // can do; its Wi-Fi link leaves when the last handle goes.
            Err(c) => drop(c),
        }
    }
}
