# Windows Manager

One app to manage a Windows 11 PC. Built with Tauri 2 (Rust) and Svelte 5, so it ships as a small native app.

**Normal mode**

- **Overview**: CPU, GPU and VRAM, memory, motherboard, drives and uptime. *Copy specs* puts a text summary on the clipboard.
- **Disk space**: scans a drive or folder in parallel and shows what is using the space as a treemap and a sorted list. You can drill into folders, open them in Explorer, or move them to the Recycle Bin.
- **Gaming**: toggles for Game Mode, Game DVR, GPU scheduling (HAGS), game priority, mouse acceleration, Sticky Keys, network throttling, transparency and the power plan. Turning a tweak off writes the Windows default back.

**Advanced mode** (switch at the bottom of the sidebar)

- **Registry**: browse, create, edit and delete keys and values. Before every change the app saves an undo `.reg` file to `%LOCALAPPDATA%\cc.astrolink.windowsmanager\registry-backups`. Double-click that file to restore. Before a key is deleted, the whole key is exported first.

Some features need administrator rights. Use *Restart as administrator* in the sidebar.

## Develop (on Windows)

Prerequisites: [Rust](https://rustup.rs), [Bun](https://bun.sh) and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) (WebView2 is already part of Windows 11).

```bash
bun install
```

```bash
bun tauri dev
```

Build an installer (`.msi` and NSIS `.exe`, written to `src-tauri/target/release/bundle`):

```bash
bun tauri build
```

Tests: `bun run test` (treemap layout) and `cargo test` in `src-tauri` (disk scanner, registry encoding).

## Layout

- `src-tauri/src/system.rs`: hardware info (sysinfo)
- `src-tauri/src/disk.rs`: parallel folder scanner, keeps the tree in memory
- `src-tauri/src/tweaks.rs`: gaming tweak table
- `src-tauri/src/registry.rs`: registry editor and undo files
- `src/lib/views/*.svelte`: one file per page
