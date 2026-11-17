//! DJI Mic Control app. The device layer and the Tauri commands are shared;
//! the shell around them differs between desktop (tray, menus, autostart)
//! and mobile (Android hands receivers over through a small plugin).

mod commands;
#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(desktop)]
    desktop::run();
    #[cfg(mobile)]
    mobile::run();
}
