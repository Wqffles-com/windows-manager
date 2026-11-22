import { invoke } from "@tauri-apps/api/core";

export type Disk = { mount: string; label: string; fs: string; kind: string; total: number; available: number; removable: boolean };
export type SystemInfo = {
  hostname: string;
  os: string;
  os_version: string;
  build: number;
  uptime: number;
  cpu: string;
  cores: number;
  threads: number;
  cpu_mhz: number;
  mem_total: number;
  mem_used: number;
  swap_total: number;
  gpus: { name: string; vram: number | null }[];
  board: string | null;
  disks: Disk[];
};
export type Entry = { name: string; size: number; files: number; is_dir: boolean };
export type Listing = { size: number; files: number; entries: Entry[] };
export type Tweak = { id: string; title: string; desc: string; category: string; admin: boolean; restart: boolean; enabled: boolean };
export type RegValue = { name: string; kind: string; data: string };
export type RegListing = { subkeys: string[]; values: RegValue[] };

export const api = {
  systemInfo: () => invoke<SystemInfo>("system_info"),
  isElevated: () => invoke<boolean>("is_elevated"),
  relaunchAsAdmin: () => invoke<void>("relaunch_as_admin"),

  scanStart: (path: string) => invoke<void>("scan_start", { path }),
  scanProgress: () => invoke<number>("scan_progress"),
  scanCancel: () => invoke<void>("scan_cancel"),
  dirChildren: (path: string[], limit = 500) => invoke<Listing>("dir_children", { path, limit }),
  entryPath: (path: string[]) => invoke<string>("entry_path", { path }),
  deleteEntry: (path: string[]) => invoke<void>("delete_entry", { path }),

  tweaks: () => invoke<Tweak[]>("tweaks_list"),
  tweakSet: (id: string, enabled: boolean) => invoke<void>("tweak_set", { id, enabled }),

  regList: (path: string) => invoke<RegListing>("reg_list", { path }),
  regSetValue: (path: string, name: string, kind: string, data: string) => invoke<string>("reg_set_value", { path, name, kind, data }),
  regDeleteValue: (path: string, name: string) => invoke<string>("reg_delete_value", { path, name }),
  regCreateKey: (path: string, name: string) => invoke<void>("reg_create_key", { path, name }),
  regDeleteKey: (path: string) => invoke<string>("reg_delete_key", { path }),
  regBackupDir: () => invoke<string>("reg_backup_dir"),
};
