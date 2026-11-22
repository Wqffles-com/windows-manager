#[cfg(not(windows))]
compile_error!("Windows Manager only targets Windows (build with --target x86_64-pc-windows-msvc).");

mod disk;
mod registry;
mod system;
mod tweaks;

use std::os::windows::process::CommandExt;
use std::process::Command;

/// A console command that does not flash a console window.
pub(crate) fn hidden(program: &str) -> Command {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut c = Command::new(program);
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

/// Reading the LocalService hive is only allowed for an elevated process.
#[tauri::command]
fn is_elevated() -> bool {
    winreg::RegKey::predef(winreg::enums::HKEY_USERS)
        .open_subkey("S-1-5-19")
        .is_ok()
}

#[tauri::command]
fn relaunch_as_admin(app: tauri::AppHandle) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy().replace('\'', "''");
    let status = hidden("powershell")
        .args(["-NoProfile", "-Command", &format!("Start-Process -FilePath '{exe}' -Verb RunAs")])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Elevation was cancelled.".into());
    }
    app.exit(0);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(std::sync::Arc::new(disk::DiskState::default()))
        .invoke_handler(tauri::generate_handler![
            is_elevated,
            relaunch_as_admin,
            system::system_info,
            disk::scan_start,
            disk::scan_progress,
            disk::scan_cancel,
            disk::dir_children,
            disk::delete_entry,
            disk::entry_path,
            tweaks::tweaks_list,
            tweaks::tweak_set,
            registry::reg_list,
            registry::reg_set_value,
            registry::reg_delete_value,
            registry::reg_create_key,
            registry::reg_delete_key,
            registry::reg_backup_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
