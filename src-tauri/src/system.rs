use serde::Serialize;
use sysinfo::{Disks, Gpus, Motherboard, System};

#[derive(Serialize)]
pub struct Gpu {
    name: String,
    vram: Option<u64>,
}

#[derive(Serialize)]
pub struct Disk {
    mount: String,
    label: String,
    fs: String,
    kind: String,
    total: u64,
    available: u64,
    removable: bool,
}

#[derive(Serialize)]
pub struct SystemInfo {
    hostname: String,
    os: String,
    os_version: String,
    build: u32,
    uptime: u64,
    cpu: String,
    cores: usize,
    threads: usize,
    cpu_mhz: u64,
    mem_total: u64,
    mem_used: u64,
    swap_total: u64,
    gpus: Vec<Gpu>,
    board: Option<String>,
    disks: Vec<Disk>,
}

#[tauri::command]
pub async fn system_info() -> Result<SystemInfo, String> {
    tauri::async_runtime::spawn_blocking(collect)
        .await
        .map_err(|e| e.to_string())?
}

fn collect() -> Result<SystemInfo, String> {
    let mut sys = System::new().map_err(|e| e.to_string())?;
    sys.refresh_memory();
    sys.refresh_cpu_all();
    let cpu = sys.cpus().first();

    let gpus = Gpus::new_with_refreshed_list()
        .map(|g| {
            g.iter()
                .filter_map(|g| {
                    let name = g.model()?.to_string();
                    // DXGI also lists the software rasterizer.
                    (!name.starts_with("Microsoft Basic")).then(|| Gpu { name, vram: g.total_memory() })
                })
                .collect()
        })
        .unwrap_or_default();

    let board = Motherboard::new().ok().and_then(|m| {
        let parts: Vec<String> = [m.vendor_name(), m.name()].into_iter().flatten().collect();
        (!parts.is_empty()).then(|| parts.join(" "))
    });

    let disks = Disks::new_with_refreshed_list()
        .map(|d| {
            d.iter()
                .map(|d| Disk {
                    mount: d.mount_point().to_string_lossy().into(),
                    label: d.name().to_string_lossy().into(),
                    fs: d.file_system().to_string_lossy().into(),
                    kind: d.kind().to_string(),
                    total: d.total_space(),
                    available: d.available_space(),
                    removable: d.is_removable(),
                })
                .collect()
        })
        .unwrap_or_default();

    // "10.0.22631" style kernel version; the last part is the build number.
    let kernel = System::kernel_version().unwrap_or_default();
    let build = kernel.rsplit('.').next().and_then(|b| b.parse().ok()).unwrap_or(0);

    Ok(SystemInfo {
        hostname: System::host_name().unwrap_or_default(),
        os: System::long_os_version().unwrap_or_default(),
        os_version: System::os_version().unwrap_or_default(),
        build,
        uptime: System::uptime().unwrap_or(0),
        cpu: cpu.map(|c| c.brand().trim().to_string()).unwrap_or_default(),
        cores: System::physical_core_count().unwrap_or(0),
        threads: sys.cpus().len(),
        cpu_mhz: cpu.map(|c| c.frequency()).unwrap_or(0),
        mem_total: sys.total_memory(),
        mem_used: sys.used_memory(),
        swap_total: sys.total_swap(),
        gpus,
        board,
        disks,
    })
}
