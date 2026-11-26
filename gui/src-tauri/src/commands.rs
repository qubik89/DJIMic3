//! Tauri commands bridging the Svelte front-end to the shared device layer.

use std::sync::Arc;

use device::{DeviceManager, DeviceStatus, DeviceSummary, Os, Probe, Setting};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

/// Ko-fi page for tips. Fixed here so the webview can't open arbitrary URLs.
pub const SUPPORT_URL: &str = "https://ko-fi.com/qubik";

/// Where users report problems (opened from the diagnostic report).
pub const ISSUES_URL: &str = "https://github.com/qubik89/DJIMic3/issues/new";

/// Everything the UI needs for one render, fetched in a single call.
#[derive(Serialize)]
pub struct Snapshot {
    /// Host operating system.
    pub os: Os,
    /// Latest bus scan (drives the setup-help banner).
    pub probe: Probe,
    /// All tracked devices for the sidebar.
    pub devices: Vec<DeviceSummary>,
    /// Full status of the selected device, if any.
    pub status: Option<DeviceStatus>,
    /// Setting descriptors for the selected device's model.
    pub settings: Vec<Setting>,
}

/// The udev-rules helper contents shown on Linux when access is blocked.
#[derive(Serialize)]
pub struct UdevHelp {
    pub file: String,
    pub rule: String,
    pub steps: Vec<String>,
}

/// Read the current state for `device` (or the sole device when omitted).
#[tauri::command]
pub fn snapshot(device: Option<String>, mgr: State<'_, Arc<DeviceManager>>) -> Snapshot {
    let devices = mgr.list();
    let probe = mgr.probe();
    let selected = device.or_else(|| mgr.only_device());

    let (status, settings) = match &selected {
        Some(id) => (
            mgr.status(id).ok(),
            mgr.settings(id).map(|s| s.to_vec()).unwrap_or_default(),
        ),
        None => (None, Vec::new()),
    };

    Snapshot {
        os: device::current_os(),
        probe,
        devices,
        status,
        settings,
    }
}

/// Change one setting on a device.
#[tauri::command]
pub fn set_setting(
    device: String,
    setting: String,
    value: String,
    mgr: State<'_, Arc<DeviceManager>>,
) -> Result<(), String> {
    mgr.set(&device, &setting, &value).map_err(|e| e.to_string())
}

/// Change one setting on a specific transmitter slot (0-based) of a device —
/// for a setting that targets one TX individually rather than mirroring
/// across both (currently just Voice Tone).
#[tauri::command]
pub fn set_tx_setting(
    device: String,
    tx: usize,
    setting: String,
    value: String,
    mgr: State<'_, Arc<DeviceManager>>,
) -> Result<(), String> {
    mgr.set_tx(&device, tx, &setting, &value).map_err(|e| e.to_string())
}

/// The udev rule and instructions for Linux setup.
#[tauri::command]
pub fn udev_help() -> UdevHelp {
    UdevHelp {
        file: device::UDEV_RULE_FILE.to_string(),
        rule: device::udev_rule(),
        steps: device::udev_instructions(),
    }
}

/// Open the Ko-fi page in the default browser. On success the webview hears
/// `support-opened` and stops showing its reminder, whichever button or menu
/// item opened the page.
pub fn open_support_page(app: &AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(SUPPORT_URL, None::<&str>)
        .map_err(|e| e.to_string())?;
    let _ = app.emit("support-opened", ());
    Ok(())
}

/// The webview's "Buy me a coffee" buttons.
#[tauri::command]
pub fn open_support(app: AppHandle) -> Result<(), String> {
    open_support_page(&app)
}

/// The troubleshooting report shown in the diagnostics window.
#[tauri::command]
pub async fn diagnostics(app: AppHandle, mgr: State<'_, Arc<DeviceManager>>) -> Result<String, String> {
    let mgr = mgr.inner().clone();
    let version = app.package_info().version.to_string();
    tauri::async_runtime::spawn_blocking(move || mgr.diagnostics(&version))
        .await
        .map_err(|e| e.to_string())
}

/// Open the project's "new issue" page in the default browser.
#[tauri::command]
pub fn open_issues(app: AppHandle) -> Result<(), String> {
    app.opener().open_url(ISSUES_URL, None::<&str>).map_err(|e| e.to_string())
}
