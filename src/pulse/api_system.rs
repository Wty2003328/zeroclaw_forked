use axum::{extract::State, response::Json, routing::get, Router};
use sysinfo::{Disks, Networks, System};

use crate::pulse::PulseState;

pub fn routes() -> Router<PulseState> {
    Router::new().route("/stats", get(system_stats))
}

async fn system_stats(State(state): State<PulseState>) -> Json<serde_json::Value> {
    let mut sys = state.sysinfo.lock().unwrap();
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    let cpu = sys.global_cpu_usage();
    let cpu_count = sys.cpus().len();
    let mem_used = sys.used_memory();
    let mem_total = sys.total_memory();
    let mem_pct = if mem_total > 0 {
        (mem_used as f64 / mem_total as f64) * 100.0
    } else {
        0.0
    };
    let swap_used = sys.used_swap();
    let swap_total = sys.total_swap();
    let swap_pct = if swap_total > 0 {
        (swap_used as f64 / swap_total as f64) * 100.0
    } else {
        0.0
    };

    let proc_count = sys.processes().len();
    let hostname = System::host_name().unwrap_or_else(|| "unknown".to_string());
    let os = System::long_os_version().unwrap_or_default();
    let kernel = System::kernel_version().unwrap_or_default();
    let uptime = System::uptime();

    drop(sys);

    let disks = Disks::new_with_refreshed_list();
    let (disk_used, disk_total) = disks.iter().fold((0u64, 0u64), |(u, t), d| {
        (
            u + (d.total_space() - d.available_space()),
            t + d.total_space(),
        )
    });
    let disk_pct = if disk_total > 0 {
        (disk_used as f64 / disk_total as f64) * 100.0
    } else {
        0.0
    };

    let networks = Networks::new_with_refreshed_list();
    let (net_rx, net_tx) = networks.iter().fold((0u64, 0u64), |(rx, tx), (_, n)| {
        (rx + n.total_received(), tx + n.total_transmitted())
    });

    Json(serde_json::json!({
        "hostname": hostname,
        "os": os,
        "kernel": kernel,
        "uptime_secs": uptime,
        "cpu_percent": (cpu as f64 * 10.0).round() / 10.0,
        "cpu_count": cpu_count,
        "memory_used_bytes": mem_used,
        "memory_total_bytes": mem_total,
        "memory_percent": (mem_pct * 10.0).round() / 10.0,
        "swap_used_bytes": swap_used,
        "swap_total_bytes": swap_total,
        "swap_percent": (swap_pct * 10.0).round() / 10.0,
        "disk_used_bytes": disk_used,
        "disk_total_bytes": disk_total,
        "disk_percent": (disk_pct * 10.0).round() / 10.0,
        "process_count": proc_count,
        "network_rx_bytes": net_rx,
        "network_tx_bytes": net_tx,
    }))
}
