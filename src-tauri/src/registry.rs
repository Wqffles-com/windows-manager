//! Registry browser/editor for advanced mode. Every write first saves an undo
//! `.reg` file (double-click it to restore) in the app's data folder.

use serde::Serialize;
use std::borrow::Cow;
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use winreg::enums::*;
use winreg::types::FromRegValue;
use winreg::{HKEY, RegKey, RegValue};

const HIVES: &[(&str, HKEY)] = &[
    ("HKEY_CLASSES_ROOT", HKEY_CLASSES_ROOT),
    ("HKEY_CURRENT_USER", HKEY_CURRENT_USER),
    ("HKEY_LOCAL_MACHINE", HKEY_LOCAL_MACHINE),
    ("HKEY_USERS", HKEY_USERS),
    ("HKEY_CURRENT_CONFIG", HKEY_CURRENT_CONFIG),
];

const TYPES: &[(&str, RegType)] = &[
    ("REG_NONE", REG_NONE),
    ("REG_SZ", REG_SZ),
    ("REG_EXPAND_SZ", REG_EXPAND_SZ),
    ("REG_BINARY", REG_BINARY),
    ("REG_DWORD", REG_DWORD),
    ("REG_DWORD_BIG_ENDIAN", REG_DWORD_BIG_ENDIAN),
    ("REG_LINK", REG_LINK),
    ("REG_MULTI_SZ", REG_MULTI_SZ),
    ("REG_RESOURCE_LIST", REG_RESOURCE_LIST),
    ("REG_FULL_RESOURCE_DESCRIPTOR", REG_FULL_RESOURCE_DESCRIPTOR),
    ("REG_RESOURCE_REQUIREMENTS_LIST", REG_RESOURCE_REQUIREMENTS_LIST),
    ("REG_QWORD", REG_QWORD),
];

#[derive(Serialize)]
pub struct Value {
    name: String,
    kind: &'static str,
    data: String,
}

#[derive(Serialize)]
pub struct Listing {
    subkeys: Vec<String>,
    values: Vec<Value>,
}

fn err(e: io::Error) -> String {
    match e.kind() {
        ErrorKind::PermissionDenied => "Access denied. Restart as administrator, or this key is protected by Windows.".into(),
        ErrorKind::NotFound => "Key or value not found.".into(),
        _ => e.to_string(),
    }
}

fn open(path: &str, flags: u32) -> io::Result<RegKey> {
    let (hive, sub) = path.split_once('\\').unwrap_or((path, ""));
    let (_, h) = HIVES
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(hive))
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "Unknown hive"))?;
    RegKey::predef(*h).open_subkey_with_flags(sub, flags)
}

fn type_name(t: &RegType) -> &'static str {
    TYPES.iter().find(|(_, v)| v == t).map(|(n, _)| *n).unwrap_or("REG_NONE")
}

fn hex(bytes: &[u8], sep: &str) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(sep)
}

/// Text form shown and edited in the UI: strings as-is, multi-strings one per
/// line, numbers in decimal, everything else as space-separated hex bytes.
fn display(v: &RegValue) -> String {
    let text = match v.vtype {
        REG_SZ | REG_EXPAND_SZ => String::from_reg_value(v).ok(),
        REG_MULTI_SZ => Vec::<String>::from_reg_value(v).ok().map(|l| l.join("\n")),
        REG_DWORD => u32::from_reg_value(v).ok().map(|n| n.to_string()),
        REG_QWORD => u64::from_reg_value(v).ok().map(|n| n.to_string()),
        _ => None,
    };
    text.unwrap_or_else(|| hex(&v.bytes, " "))
}

fn utf16z(s: &str) -> Vec<u8> {
    s.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect()
}

fn parse_num(s: &str) -> Result<u64, String> {
    let s = s.trim();
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(h) => u64::from_str_radix(h, 16),
        None => s.parse(),
    }
    .map_err(|_| format!("\"{s}\" is not a number"))
}

/// Inverse of `display`.
fn parse(kind: &str, data: &str) -> Result<RegValue<'static>, String> {
    let (_, vtype) = TYPES.iter().find(|(n, _)| *n == kind).ok_or("Unknown value type")?;
    let bytes = match *vtype {
        REG_SZ | REG_EXPAND_SZ => utf16z(data),
        REG_MULTI_SZ => {
            let mut b: Vec<u8> = data.lines().filter(|l| !l.is_empty()).flat_map(utf16z).collect();
            b.extend([0, 0]);
            b
        }
        REG_DWORD => {
            let n = parse_num(data)?;
            u32::try_from(n).map_err(|_| "Too large for a DWORD (max 4294967295)")?.to_le_bytes().to_vec()
        }
        REG_QWORD => parse_num(data)?.to_le_bytes().to_vec(),
        _ => data
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .map(|b| u8::from_str_radix(b, 16).map_err(|_| format!("\"{b}\" is not a hex byte")))
            .collect::<Result<_, _>>()?,
    };
    Ok(RegValue { bytes: Cow::Owned(bytes), vtype: vtype.clone() })
}

fn backup_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("registry-backups");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn backup_file(app: &AppHandle, path: &str) -> Result<PathBuf, String> {
    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
    let leaf: String = path.rsplit('\\').next().unwrap_or("").chars().filter(|c| c.is_alphanumeric()).take(40).collect();
    Ok(backup_dir(app)?.join(format!("{ms}-{leaf}.reg")))
}

/// `.reg` text that puts one value back the way it was (or deletes it if it did not exist).
fn undo_reg(path: &str, name: &str, old: Option<&RegValue>) -> String {
    let name = if name.is_empty() { "@".into() } else { format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\"")) };
    let data = match old {
        None => "-".into(),
        Some(v) => format!("hex({:x}):{}", v.vtype.clone() as u32, hex(&v.bytes, ",")),
    };
    format!("Windows Registry Editor Version 5.00\r\n\r\n[{path}]\r\n{name}={data}\r\n")
}

fn save_undo(app: &AppHandle, path: &str, name: &str, old: Option<&RegValue>) -> Result<String, String> {
    let file = backup_file(app, path)?;
    // regedit expects UTF-16LE with a BOM for "Version 5.00" files.
    let bytes: Vec<u8> = [0xFF, 0xFE].into_iter().chain(undo_reg(path, name, old).encode_utf16().flat_map(u16::to_le_bytes)).collect();
    std::fs::write(&file, bytes).map_err(|e| e.to_string())?;
    Ok(file.to_string_lossy().into())
}

#[tauri::command]
pub fn reg_list(path: String) -> Result<Listing, String> {
    if path.is_empty() {
        return Ok(Listing { subkeys: HIVES.iter().map(|(n, _)| n.to_string()).collect(), values: vec![] });
    }
    let key = open(&path, KEY_READ).map_err(err)?;
    let mut subkeys: Vec<String> = key.enum_keys().filter_map(Result::ok).collect();
    subkeys.sort_unstable_by_key(|s| s.to_lowercase());
    let mut values: Vec<Value> = key
        .enum_values()
        .filter_map(Result::ok)
        .map(|(name, v)| Value { data: display(&v), kind: type_name(&v.vtype), name })
        .collect();
    values.sort_unstable_by_key(|v| v.name.to_lowercase());
    Ok(Listing { subkeys, values })
}

/// Returns the path of the undo file.
#[tauri::command]
pub fn reg_set_value(app: AppHandle, path: String, name: String, kind: String, data: String) -> Result<String, String> {
    let value = parse(&kind, &data)?;
    let key = open(&path, KEY_READ | KEY_WRITE).map_err(err)?;
    let undo = save_undo(&app, &path, &name, key.get_raw_value(&name).ok().as_ref())?;
    key.set_raw_value(&name, &value).map_err(err)?;
    Ok(undo)
}

#[tauri::command]
pub fn reg_delete_value(app: AppHandle, path: String, name: String) -> Result<String, String> {
    let key = open(&path, KEY_READ | KEY_WRITE).map_err(err)?;
    let old = key.get_raw_value(&name).map_err(err)?;
    let undo = save_undo(&app, &path, &name, Some(&old))?;
    key.delete_value(&name).map_err(err)?;
    Ok(undo)
}

#[tauri::command]
pub fn reg_create_key(path: String, name: String) -> Result<(), String> {
    if name.is_empty() || name.contains('\\') {
        return Err("Key name cannot be empty or contain \\".into());
    }
    open(&path, KEY_READ | KEY_WRITE).map_err(err)?.create_subkey(&name).map_err(err)?;
    Ok(())
}

/// Exports the whole key with `reg export` first; refuses to delete if that fails.
#[tauri::command]
pub fn reg_delete_key(app: AppHandle, path: String) -> Result<String, String> {
    let (parent, leaf) = path.rsplit_once('\\').ok_or("Cannot delete a hive")?;
    if !parent.contains('\\') {
        return Err("Refusing to delete a top-level key like HKEY_LOCAL_MACHINE\\SOFTWARE".into());
    }
    let file = backup_file(&app, &path)?;
    let out = crate::hidden("reg").arg("export").arg(&path).arg(&file).arg("/y").output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("Backup failed, key not deleted: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    open(parent, KEY_READ | KEY_WRITE).map_err(err)?.delete_subkey_all(leaf).map_err(err)?;
    Ok(file.to_string_lossy().into())
}

#[tauri::command]
pub fn reg_backup_dir(app: AppHandle) -> Result<String, String> {
    Ok(backup_dir(&app)?.to_string_lossy().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_display_roundtrip() {
        for (kind, text) in [
            ("REG_SZ", "hello wörld"),
            ("REG_EXPAND_SZ", "%SystemRoot%\\x"),
            ("REG_MULTI_SZ", "a\nbc"),
            ("REG_DWORD", "4294967295"),
            ("REG_QWORD", "18446744073709551615"),
            ("REG_BINARY", "00 ff 10"),
        ] {
            assert_eq!(display(&parse(kind, text).unwrap()), text, "{kind}");
        }
        assert_eq!(display(&parse("REG_DWORD", "0x10").unwrap()), "16");
        assert!(parse("REG_DWORD", "4294967296").is_err());
        assert!(parse("REG_BINARY", "zz").is_err());
    }

    #[test]
    fn undo_file_format() {
        let v = parse("REG_DWORD", "1").unwrap();
        assert_eq!(undo_reg(r"HKEY_CURRENT_USER\X", "a\"b", Some(&v)), "Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER\\X]\r\n\"a\\\"b\"=hex(4):01,00,00,00\r\n");
        assert!(undo_reg(r"HKEY_CURRENT_USER\X", "", None).ends_with("@=-\r\n"));
    }
}
