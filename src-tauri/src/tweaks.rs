//! Gaming/performance tweaks. Each tweak is a toggle: "on" writes the tuned
//! values, "off" writes the Windows defaults back.

use serde::Serialize;
use winreg::enums::{HKEY_CURRENT_USER as HKCU, HKEY_LOCAL_MACHINE as HKLM};
use winreg::{HKEY, RegKey};

#[derive(Clone, Copy)]
enum V {
    D(u32),
    S(&'static str),
}

/// One registry value: `on` = tweaked, `off` = Windows default, `absent` = what Windows assumes when it is missing.
struct Reg {
    hive: HKEY,
    key: &'static str,
    name: &'static str,
    on: V,
    off: V,
    absent: V,
}

struct Tweak {
    id: &'static str,
    title: &'static str,
    desc: &'static str,
    category: &'static str,
    admin: bool,
    restart: bool,
    regs: &'static [Reg],
}

const MM: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";
const MM_GAMES: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games";

const TWEAKS: &[Tweak] = &[
    Tweak {
        id: "game_mode",
        title: "Game Mode",
        desc: "Lets Windows prioritise the game you are playing and pause background work like Windows Update installs.",
        category: "Gaming",
        admin: false,
        restart: false,
        regs: &[Reg { hive: HKCU, key: r"Software\Microsoft\GameBar", name: "AutoGameModeEnabled", on: V::D(1), off: V::D(0), absent: V::D(1) }],
    },
    Tweak {
        id: "game_dvr_off",
        title: "Disable background recording",
        desc: "Turns off Xbox Game Bar background capture (Game DVR), which costs FPS while it records.",
        category: "Gaming",
        admin: false,
        restart: false,
        regs: &[
            Reg { hive: HKCU, key: r"System\GameConfigStore", name: "GameDVR_Enabled", on: V::D(0), off: V::D(1), absent: V::D(1) },
            Reg { hive: HKCU, key: r"Software\Microsoft\Windows\CurrentVersion\GameDVR", name: "AppCaptureEnabled", on: V::D(0), off: V::D(1), absent: V::D(1) },
        ],
    },
    Tweak {
        id: "hags",
        title: "Hardware-accelerated GPU scheduling",
        desc: "Lets the GPU manage its own memory scheduling. Lowers latency on most modern GPUs; required for DLSS Frame Generation.",
        category: "Gaming",
        admin: true,
        restart: true,
        regs: &[Reg { hive: HKLM, key: r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers", name: "HwSchMode", on: V::D(2), off: V::D(1), absent: V::D(1) }],
    },
    Tweak {
        id: "game_priority",
        title: "Higher CPU & I/O priority for games",
        desc: "Raises the scheduling priority Windows gives to games registered with the multimedia scheduler.",
        category: "Gaming",
        admin: true,
        restart: true,
        regs: &[
            Reg { hive: HKLM, key: MM_GAMES, name: "Priority", on: V::D(6), off: V::D(2), absent: V::D(2) },
            Reg { hive: HKLM, key: MM_GAMES, name: "Scheduling Category", on: V::S("High"), off: V::S("Medium"), absent: V::S("Medium") },
            Reg { hive: HKLM, key: MM_GAMES, name: "SFIO Priority", on: V::S("High"), off: V::S("Normal"), absent: V::S("Normal") },
        ],
    },
    Tweak {
        id: "mouse_accel_off",
        title: "Disable mouse acceleration",
        desc: "Turns off \"Enhance pointer precision\" so mouse movement is 1:1, which aim relies on.",
        category: "Input",
        admin: false,
        restart: true,
        regs: &[
            Reg { hive: HKCU, key: r"Control Panel\Mouse", name: "MouseSpeed", on: V::S("0"), off: V::S("1"), absent: V::S("1") },
            Reg { hive: HKCU, key: r"Control Panel\Mouse", name: "MouseThreshold1", on: V::S("0"), off: V::S("6"), absent: V::S("6") },
            Reg { hive: HKCU, key: r"Control Panel\Mouse", name: "MouseThreshold2", on: V::S("0"), off: V::S("10"), absent: V::S("10") },
        ],
    },
    Tweak {
        id: "sticky_keys_off",
        title: "Disable Sticky Keys shortcut",
        desc: "Stops the Sticky Keys popup when you press Shift five times mid-game.",
        category: "Input",
        admin: false,
        restart: true,
        regs: &[Reg { hive: HKCU, key: r"Control Panel\Accessibility\StickyKeys", name: "Flags", on: V::S("506"), off: V::S("510"), absent: V::S("510") }],
    },
    Tweak {
        id: "net_throttle_off",
        title: "Disable network throttling",
        desc: "Stops Windows from limiting network packet processing while media plays, and reserves less CPU for background tasks.",
        category: "Network",
        admin: true,
        restart: true,
        regs: &[
            Reg { hive: HKLM, key: MM, name: "NetworkThrottlingIndex", on: V::D(0xFFFF_FFFF), off: V::D(10), absent: V::D(10) },
            Reg { hive: HKLM, key: MM, name: "SystemResponsiveness", on: V::D(10), off: V::D(20), absent: V::D(20) },
        ],
    },
    Tweak {
        id: "transparency_off",
        title: "Disable transparency effects",
        desc: "Turns off the see-through blur on the taskbar and Start menu. Small GPU saving on low-end systems.",
        category: "Visual",
        admin: false,
        restart: false,
        regs: &[Reg { hive: HKCU, key: r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", name: "EnableTransparency", on: V::D(0), off: V::D(1), absent: V::D(1) }],
    },
];

const HIGH_PERF: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
const ULTIMATE: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";

#[derive(Serialize)]
pub struct TweakState {
    id: &'static str,
    title: &'static str,
    desc: &'static str,
    category: &'static str,
    admin: bool,
    restart: bool,
    enabled: bool,
}

fn is_on(r: &Reg) -> bool {
    let key = RegKey::predef(r.hive).open_subkey(r.key).ok();
    match (r.on, r.absent) {
        (V::D(on), V::D(absent)) => key.and_then(|k| k.get_value::<u32, _>(r.name).ok()).unwrap_or(absent) == on,
        (V::S(on), V::S(absent)) => key.and_then(|k| k.get_value::<String, _>(r.name).ok()).as_deref().unwrap_or(absent) == on,
        _ => false,
    }
}

fn write(r: &Reg, v: V) -> std::io::Result<()> {
    let (key, _) = RegKey::predef(r.hive).create_subkey(r.key)?;
    match v {
        V::D(d) => key.set_value(r.name, &d),
        V::S(s) => key.set_value(r.name, &s),
    }
}

fn active_power_plan() -> String {
    crate::hidden("powercfg")
        .arg("/getactivescheme")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase())
        .unwrap_or_default()
}

#[tauri::command]
pub fn tweaks_list() -> Vec<TweakState> {
    let plan = active_power_plan();
    let mut list: Vec<TweakState> = TWEAKS
        .iter()
        .map(|t| TweakState {
            id: t.id,
            title: t.title,
            desc: t.desc,
            category: t.category,
            admin: t.admin,
            restart: t.restart,
            enabled: t.regs.iter().all(is_on),
        })
        .collect();
    list.push(TweakState {
        id: "power_plan",
        title: "High performance power plan",
        desc: "Keeps the CPU at high clocks instead of saving power. Uses more energy; not recommended on battery.",
        category: "Power",
        admin: false,
        restart: false,
        enabled: plan.contains(HIGH_PERF) || plan.contains(ULTIMATE),
    });
    list
}

#[tauri::command]
pub fn tweak_set(id: String, enabled: bool) -> Result<(), String> {
    if id == "power_plan" {
        // Aliases work on every Windows language; SCHEME_MIN is "High performance".
        let scheme = if enabled { "SCHEME_MIN" } else { "SCHEME_BALANCED" };
        let out = crate::hidden("powercfg").args(["/setactive", scheme]).output().map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err("This PC has no High performance plan (common on laptops with Modern Standby).".into());
        }
        return Ok(());
    }
    let t = TWEAKS.iter().find(|t| t.id == id).ok_or("Unknown tweak")?;
    for r in t.regs {
        write(r, if enabled { r.on } else { r.off }).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                "Administrator rights needed. Restart the app as administrator.".to_string()
            } else {
                e.to_string()
            }
        })?;
    }
    Ok(())
}
