use procfs::{Current, Meminfo};

use crate::runtime::signal_bus::SignalBus;

pub fn collect(bus: &SignalBus) -> Result<(), String> {
    let mem = Meminfo::current().map_err(|e| e.to_string())?;

    let total_bytes = mem.mem_total;
    let available_bytes = mem.mem_available.unwrap_or(mem.mem_free);
    if total_bytes == 0 {
        return Ok(());
    }
    let used_bytes = total_bytes.saturating_sub(available_bytes);

    let percent = ((used_bytes as f64 / total_bytes as f64) * 100.0).clamp(0.0, 100.0);

    bus.set("data.memory.percent", &format!("{:.0}", percent));
    bus.set(
        "data.memory.state",
        if percent >= 90.0 {
            "critical"
        } else if percent >= 75.0 {
            "warn"
        } else {
            "normal"
        },
    );

    bus.set(
        "data.memory.total_mb",
        &(total_bytes / 1024 / 1024).to_string(),
    );
    bus.set(
        "data.memory.used_mb",
        &(used_bytes / 1024 / 1024).to_string(),
    );
    bus.set(
        "data.memory.available_mb",
        &(available_bytes / 1024 / 1024).to_string(),
    );
    bus.set(
        "data.memory.total_gb",
        &format!("{:.1}", total_bytes as f64 / 1024.0 / 1024.0 / 1024.0),
    );
    bus.set(
        "data.memory.used_gb",
        &format!("{:.1}", used_bytes as f64 / 1024.0 / 1024.0 / 1024.0),
    );
    bus.set(
        "data.memory.available_gb",
        &format!("{:.1}", available_bytes as f64 / 1024.0 / 1024.0 / 1024.0),
    );

    Ok(())
}
