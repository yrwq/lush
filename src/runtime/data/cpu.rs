use procfs::{CurrentSI, KernelStats};

use super::{CpuTotals, CpuTotalsState};
use crate::runtime::signal_bus::SignalBus;

fn totals(stats: &KernelStats) -> (u64, u64, u64, u64) {
    let total = &stats.total;
    let user = total.user.saturating_add(total.nice);
    let system = total.system;
    let idle = total.idle.saturating_add(total.iowait.unwrap_or(0));
    let total_ticks = user
        .saturating_add(system)
        .saturating_add(idle)
        .saturating_add(total.irq.unwrap_or(0))
        .saturating_add(total.softirq.unwrap_or(0))
        .saturating_add(total.steal.unwrap_or(0))
        .saturating_add(total.guest.unwrap_or(0))
        .saturating_add(total.guest_nice.unwrap_or(0));
    (user, system, idle, total_ticks)
}

pub fn collect(bus: &SignalBus, cpu_prev_totals: &CpuTotalsState) -> Result<(), String> {
    let stats = KernelStats::current().map_err(|e| e.to_string())?;
    let (user, system, idle, total_ticks) = totals(&stats);

    if let Some(prev) = *cpu_prev_totals.borrow() {
        let delta_total = total_ticks.saturating_sub(prev.total_ticks);
        let delta_idle = idle.saturating_sub(prev.idle_ticks);
        if delta_total > 0 {
            let used: f64 = 1.0 - (delta_idle as f64 / delta_total as f64);
            let percent = (used * 100.0).clamp(0.0, 100.0);
            bus.set("data.cpu.percent", &format!("{:.0}", percent));
            bus.set(
                "data.cpu.state",
                if percent >= 90.0 {
                    "critical"
                } else if percent >= 70.0 {
                    "warn"
                } else {
                    "normal"
                },
            );
        }
    } else {
        let used = if total_ticks > 0 {
            1.0 - (idle as f64 / total_ticks as f64)
        } else {
            0.0
        };
        let percent = (used * 100.0).clamp(0.0, 100.0);
        bus.set("data.cpu.percent", &format!("{:.0}", percent));
        bus.set(
            "data.cpu.state",
            if percent >= 90.0 {
                "critical"
            } else if percent >= 70.0 {
                "warn"
            } else {
                "normal"
            },
        );
    }

    *cpu_prev_totals.borrow_mut() = Some(CpuTotals {
        total_ticks,
        idle_ticks: idle,
    });

    bus.set("data.cpu.user", &user.to_string());
    bus.set("data.cpu.system", &system.to_string());
    bus.set("data.cpu.idle", &idle.to_string());
    bus.set("data.cpu.total", &total_ticks.to_string());

    Ok(())
}
