//! Importing the transmitters' internal recordings.
//!
//! The transmitters mount as USB drives when their charging case is plugged
//! in (see the `recordings` crate). The webview polls [`recordings_scan`],
//! starts an import with [`recordings_import`] and hears
//! `recordings-progress` / `recordings-done` while it runs on its own thread.
//! Every path the webview sends back is checked against a fresh scan, so it
//! can only copy, delete or eject what is on a connected transmitter.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use recordings::{Deleted, Outcome, Progress, Scanner, Transmitter};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

#[derive(Default)]
pub struct RecordingsState {
    scanner: Scanner,
    busy: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
}

/// Transmitter drives that are mounted now, with `imported` set against `dest`.
#[tauri::command]
pub fn recordings_scan(dest: Option<String>, state: State<'_, RecordingsState>) -> Vec<Transmitter> {
    let mut txs = state.scanner.scan();
    if let Some(dest) = dest.filter(|d| !d.is_empty()) {
        recordings::mark_imported(&mut txs, Path::new(&dest));
    }
    txs
}

/// `~/Music/DJI Mic` (or the platform's equivalent).
#[tauri::command]
pub fn recordings_default_dest(app: AppHandle) -> String {
    let base = app
        .path()
        .audio_dir()
        .or_else(|_| app.path().home_dir())
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("DJI Mic").to_string_lossy().into_owned()
}

/// Ask for an import folder. `None` when the dialog is dismissed.
#[tauri::command]
pub async fn recordings_pick_dest(app: AppHandle, current: Option<String>) -> Option<String> {
    let mut dialog = app.dialog().file();
    if let Some(dir) = current.map(PathBuf::from) {
        // Start in the current folder, or its parent if it doesn't exist yet.
        if let Some(start) = dir.ancestors().find(|p| p.is_dir()) {
            dialog = dialog.set_directory(start);
        }
    }
    dialog
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

/// Copy `files` (paths on the transmitters) into `dest` in the background.
#[tauri::command]
pub fn recordings_import(
    app: AppHandle,
    files: Vec<String>,
    dest: String,
    state: State<'_, RecordingsState>,
) -> Result<(), String> {
    if dest.trim().is_empty() {
        return Err("No import folder".into());
    }
    let files: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
    let jobs = recordings::plan(&state.scanner.scan(), &files, Path::new(&dest))?;
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("An import is already running".into());
    }
    state.cancel.store(false, Ordering::SeqCst);
    let busy = state.busy.clone();
    let cancel = state.cancel.clone();
    std::thread::spawn(move || {
        let mut last = Instant::now() - Duration::from_secs(1);
        let outcome: Outcome = recordings::run(&jobs, &cancel, |p: &Progress| {
            let finished = p.done_bytes == p.total_bytes;
            if finished || last.elapsed() >= Duration::from_millis(100) {
                last = Instant::now();
                let _ = app.emit("recordings-progress", p);
            }
        });
        busy.store(false, Ordering::SeqCst);
        let _ = app.emit("recordings-done", &outcome);
    });
    Ok(())
}

/// Stop the running import after the current chunk.
#[tauri::command]
pub fn recordings_cancel(state: State<'_, RecordingsState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

/// Delete `files` from the transmitters, only those already in `dest`.
#[tauri::command]
pub fn recordings_delete(files: Vec<String>, dest: String, state: State<'_, RecordingsState>) -> Result<Deleted, String> {
    if state.busy.load(Ordering::SeqCst) {
        return Err("Wait for the import to finish".into());
    }
    let files: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
    Ok(recordings::delete_imported(&state.scanner.scan(), &files, Path::new(&dest)))
}

/// Open the import folder in the file manager, creating it if needed.
#[tauri::command]
pub fn recordings_open_dest(app: AppHandle, dest: String) -> Result<(), String> {
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    app.opener().open_path(dest, None::<&str>).map_err(|e| e.to_string())
}

/// Safely remove a transmitter drive.
#[tauri::command]
pub async fn recordings_eject(mount: String, app: AppHandle) -> Result<(), String> {
    let state = app.state::<RecordingsState>();
    if state.busy.load(Ordering::SeqCst) {
        return Err("Wait for the import to finish".into());
    }
    let mount = PathBuf::from(mount);
    if !state.scanner.scan().iter().any(|tx| tx.mount == mount) {
        return Err("Not a connected transmitter".into());
    }
    eject(&mount)
}

fn run(cmd: &mut Command) -> Result<(), String> {
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() { format!("exit status {}", out.status) } else { err })
    }
}

#[cfg(target_os = "macos")]
fn eject(mount: &Path) -> Result<(), String> {
    run(Command::new("/usr/sbin/diskutil").arg("eject").arg(mount))
}

#[cfg(windows)]
fn eject(mount: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // "E:\" → "E:"; the Explorer "Eject" verb flushes and removes the drive.
    let drive: String = mount.to_string_lossy().chars().take(2).collect();
    if !(drive.len() == 2 && drive.ends_with(':') && drive.chars().next().is_some_and(|c| c.is_ascii_alphabetic())) {
        return Err("Unexpected drive path".into());
    }
    let script = format!("(New-Object -ComObject Shell.Application).Namespace(17).ParseName('{drive}').InvokeVerb('Eject')");
    run(Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn eject(mount: &Path) -> Result<(), String> {
    // udisks-mounted drives (the desktop default) unmount without root.
    run(Command::new("gio").args(["mount", "-e"]).arg(mount)).or_else(|_| run(Command::new("umount").arg(mount)))
}
