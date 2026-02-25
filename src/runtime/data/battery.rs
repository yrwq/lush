use std::fs;
use std::path::{Path, PathBuf};

use super::evented::EventedProvider;
use crate::runtime::signal_bus::SignalBus;

#[derive(Debug)]
pub struct BatteryWatcher {
    _evented: EventedProvider,
}

fn read_u64(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse::<u64>().ok()
}

fn read_i64(path: &Path) -> Option<i64> {
    fs::read_to_string(path).ok()?.trim().parse::<i64>().ok()
}

fn read_string(path: &Path) -> Option<String> {
    Some(fs::read_to_string(path).ok()?.trim().to_string())
}

fn list_power_supply_dirs() -> Vec<PathBuf> {
    let base = Path::new("/sys/class/power_supply");
    let Ok(entries) = fs::read_dir(base) else {
        return Vec::new();
    };
    entries.flatten().map(|e| e.path()).collect()
}

fn find_battery_dir() -> Option<PathBuf> {
    for path in list_power_supply_dirs() {
        let ty = read_string(&path.join("type")).unwrap_or_default();
        if ty == "Battery" {
            return Some(path);
        }
    }
    None
}

fn energy_now_wh(dir: &Path) -> Option<f64> {
    if let Some(v) = read_u64(&dir.join("energy_now")) {
        return Some(v as f64 / 1_000_000.0);
    }

    let charge_now = read_u64(&dir.join("charge_now"))? as f64;
    let voltage_now = read_u64(&dir.join("voltage_now"))? as f64;
    Some((charge_now * voltage_now) / 1_000_000_000_000.0)
}

fn energy_full_wh(dir: &Path) -> Option<f64> {
    if let Some(v) = read_u64(&dir.join("energy_full")) {
        return Some(v as f64 / 1_000_000.0);
    }

    let charge_full = read_u64(&dir.join("charge_full"))? as f64;
    let voltage_now = read_u64(&dir.join("voltage_now"))? as f64;
    Some((charge_full * voltage_now) / 1_000_000_000_000.0)
}

fn power_watts(dir: &Path) -> Option<f64> {
    if let Some(v) = read_i64(&dir.join("power_now")) {
        return Some(v as f64 / 1_000_000.0);
    }

    let current_now = read_i64(&dir.join("current_now"))? as f64;
    let voltage_now = read_u64(&dir.join("voltage_now"))? as f64;
    Some((current_now * voltage_now) / 1_000_000_000_000.0)
}

fn ac_online() -> Option<bool> {
    let mut saw_online = false;
    for path in list_power_supply_dirs() {
        let ty = read_string(&path.join("type")).unwrap_or_default();
        if ty != "Mains" && ty != "USB" && ty != "USB_C" {
            continue;
        }
        if let Some(v) = read_u64(&path.join("online")) {
            saw_online = true;
            if v > 0 {
                return Some(true);
            }
        }
    }
    if saw_online { Some(false) } else { None }
}

fn inferred_state_when_unknown(
    previous_state: Option<&str>,
    percent_value: f64,
    ac_is_online: Option<bool>,
) -> &'static str {
    if let Some(online) = ac_is_online {
        if percent_value >= 99.5 {
            return "full";
        }
        return if online { "charging" } else { "discharging" };
    }

    match previous_state {
        Some("charging") => "charging",
        Some("discharging") => "discharging",
        Some("full") => "full",
        Some("unavailable") => "unavailable",
        _ => "unknown",
    }
}

#[derive(Clone, Debug)]
struct BatterySnapshot {
    percent: String,
    state: String,
    time_left_min: String,
    power_w: String,
}

fn collect_snapshot(previous_state: Option<&str>) -> Result<BatterySnapshot, String> {
    let Some(dir) = find_battery_dir() else {
        return Ok(BatterySnapshot {
            percent: String::new(),
            state: "unavailable".to_string(),
            time_left_min: String::new(),
            power_w: String::new(),
        });
    };

    let status_raw = read_string(&dir.join("status")).unwrap_or_else(|| "Unknown".to_string());

    let percent = {
        let from_energy = energy_now_wh(&dir)
            .zip(energy_full_wh(&dir))
            .and_then(|(now, full)| {
                if full > 0.0 {
                    Some((now / full) * 100.0)
                } else {
                    None
                }
            });
        from_energy
            .or_else(|| read_u64(&dir.join("capacity")).map(|v| v as f64))
            .map(|v| v.clamp(0.0, 100.0))
    };

    let power = power_watts(&dir).map(|v| v.abs());
    let energy_now = energy_now_wh(&dir);
    let energy_full = energy_full_wh(&dir);
    let percent_value = percent.unwrap_or(0.0);
    let ac_is_online = ac_online();

    let state = match status_raw.as_str() {
        "Charging" => "charging",
        "Discharging" => "discharging",
        "Full" => {
            if percent_value < 99.5 {
                if ac_is_online == Some(true) {
                    "charging"
                } else {
                    "discharging"
                }
            } else {
                "full"
            }
        }
        _ => inferred_state_when_unknown(previous_state, percent_value, ac_is_online),
    };

    let time_left_min = match state {
        "discharging" => match (energy_now, power) {
            (Some(now_wh), Some(w)) if w > 0.0 => Some(((now_wh / w) * 60.0).round() as u64),
            _ => None,
        },
        "charging" => match (energy_now, energy_full, power) {
            (Some(now_wh), Some(full_wh), Some(w)) if w > 0.0 && full_wh > now_wh => {
                Some((((full_wh - now_wh) / w) * 60.0).round() as u64)
            }
            _ => None,
        },
        _ => None,
    };

    Ok(BatterySnapshot {
        percent: percent.map(|v| format!("{:.0}", v)).unwrap_or_default(),
        state: state.to_string(),
        time_left_min: time_left_min.map(|v| v.to_string()).unwrap_or_default(),
        power_w: power.map(|v| format!("{:.2}", v)).unwrap_or_default(),
    })
}

fn apply_snapshot(bus: &SignalBus, snapshot: &BatterySnapshot) {
    bus.batch(|| {
        bus.set("data.battery.percent", &snapshot.percent);
        bus.set("data.battery.state", &snapshot.state);
        bus.set("data.battery.time_left_min", &snapshot.time_left_min);
        bus.set("data.battery.power_w", &snapshot.power_w);
    });
}

pub fn start_evented(bus: SignalBus) -> Result<BatteryWatcher, String> {
    let (snapshot_tx, snapshot_rx) = async_channel::unbounded::<BatterySnapshot>();
    let bus_for_snapshots = bus.clone();
    super::evented::spawn_snapshot_drain(snapshot_rx, move |snapshot| {
        apply_snapshot(&bus_for_snapshots, &snapshot);
    });

    let (trigger_tx, trigger_rx) = super::evented::trigger_channel();
    let mut evented = EventedProvider::new();
    let stop = evented.stop_flag();
    let mut previous_state: Option<String> = None;
    let collector_thread = super::evented::spawn_periodic_collector(
        "data.battery",
        stop.clone(),
        trigger_rx,
        std::time::Duration::from_secs(30),
        snapshot_tx,
        move |_reason| {
            let snapshot = collect_snapshot(previous_state.as_deref())?;
            previous_state = Some(snapshot.state.clone());
            Ok(snapshot)
        },
    );
    let event_thread = super::evented::spawn_udev_trigger_thread(stop, "power_supply", trigger_tx);
    evented.push_thread(collector_thread);
    evented.push_thread(event_thread);

    Ok(BatteryWatcher { _evented: evented })
}
