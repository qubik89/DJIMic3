//! Mobile shell. Android doesn't let apps scan USB themselves: the
//! `UsbPlugin` Kotlin class (gen/android/app/src/main/java/org/djimic/control)
//! asks the user for access to each DJI receiver, opens it, and returns its
//! file descriptor, which is handed to the shared device manager.

use std::sync::Arc;
use std::time::Duration;

use device::DeviceManager;
use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder as PluginBuilder, PluginHandle, TauriPlugin};
use tauri::{AppHandle, Manager, Runtime};

use crate::commands;

/// One receiver the plugin has opened.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // fields are only read on Android
struct OpenedDevice {
    fd: i32,
    vendor_id: u16,
    product_id: u16,
    serial: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenedDevices {
    devices: Vec<OpenedDevice>,
}

struct Usb<R: Runtime>(PluginHandle<R>);
struct SystemBars<R: Runtime>(PluginHandle<R>);

#[derive(Serialize)]
#[allow(dead_code)] // only sent on Android
struct BarStyle {
    dark: bool,
    color: String,
}

fn usb_plugin<R: Runtime>() -> TauriPlugin<R> {
    PluginBuilder::new("djiusb")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle = api.register_android_plugin("org.djimic.control", "UsbPlugin")?;
                app.manage(Usb(handle));
            }
            #[cfg(not(target_os = "android"))]
            let _ = (app, api);
            Ok(())
        })
        .build()
}

/// A plugin maps to one Kotlin class, so the system-bar styling gets its own.
fn system_bars_plugin<R: Runtime>() -> TauriPlugin<R> {
    PluginBuilder::new("systembars")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle = api.register_android_plugin("org.djimic.control", "SystemBarsPlugin")?;
                app.manage(SystemBars(handle));
            }
            #[cfg(not(target_os = "android"))]
            let _ = (app, api);
            Ok(())
        })
        .build()
}

/// Desktop translates its tray and app menus here; mobile has neither.
#[tauri::command]
fn set_locale(locale: String) -> Result<(), String> {
    let _ = locale;
    Ok(())
}

/// Match the status and navigation bars to the app's light/dark theme and
/// background colour.
#[tauri::command]
fn set_system_bars<R: Runtime>(app: AppHandle<R>, dark: bool, color: String) -> Result<(), String> {
    if let Some(bars) = app.try_state::<SystemBars<R>>() {
        bars.0
            .run_mobile_plugin::<serde::de::IgnoredAny>("setSystemBars", BarStyle { dark, color })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Take over a receiver the plugin opened. The plugin keeps its own
/// connection (and file descriptor) open, so the device layer gets a
/// duplicate that it owns and closes.
#[cfg(target_os = "android")]
fn adopt(mgr: &DeviceManager, d: OpenedDevice) {
    use std::os::fd::{FromRawFd, OwnedFd};
    let dup = unsafe { libc::dup(d.fd) };
    if dup < 0 {
        return;
    }
    let fd = unsafe { OwnedFd::from_raw_fd(dup) };
    if let Err(e) = mgr.adopt_fd(fd, d.vendor_id, d.product_id, d.serial.as_deref()) {
        eprintln!("djimic: could not open receiver: {e}");
    }
}

#[cfg(not(target_os = "android"))]
fn adopt(_mgr: &DeviceManager, _d: OpenedDevice) {}

pub fn run() {
    let manager = Arc::new(DeviceManager::new());
    let scanner = manager.clone();

    tauri::Builder::default()
        .plugin(usb_plugin())
        .plugin(system_bars_plugin())
        .plugin(tauri_plugin_opener::init())
        .manage(manager)
        .setup(move |app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                scanner.refresh();
                if let Some(usb) = handle.try_state::<Usb<tauri::Wry>>() {
                    match usb.0.run_mobile_plugin::<OpenedDevices>("openDevices", ()) {
                        Ok(list) => {
                            for d in list.devices {
                                adopt(&scanner, d);
                            }
                        }
                        Err(e) => eprintln!("djimic: USB plugin error: {e}"),
                    }
                }
                std::thread::sleep(Duration::from_millis(1000));
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_locale,
            set_system_bars,
            commands::snapshot,
            commands::set_setting,
            commands::set_tx_setting,
            commands::udev_help,
            commands::open_support,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DJI Mic Control");
}
