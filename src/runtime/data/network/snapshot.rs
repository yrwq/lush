use std::time::Instant;

use crate::runtime::signal_bus::SignalBus;

use super::super::NetworkTotals;
use super::sys::{default_route_iface, read_totals};
use super::wireless::{nm_ssid_strength, read_ssid, wireless_iface_stats};

#[derive(Clone, Debug)]
pub(super) struct WirelessSnapshot {
    pub(super) iface_override: Option<String>,
    pub(super) ssid: String,
    pub(super) strength_percent: String,
    pub(super) signal_dbm: String,
}

#[derive(Clone, Debug)]
pub(super) struct NetworkSnapshot {
    pub(super) iface: String,
    pub(super) down_bps: String,
    pub(super) up_bps: String,
    pub(super) down_kibps: String,
    pub(super) up_kibps: String,
    pub(super) state: String,
    pub(super) down_total_bytes: String,
    pub(super) up_total_bytes: String,
    pub(super) wireless: Option<WirelessSnapshot>,
}

pub(super) fn collect_snapshot(
    iface_filter: Option<&str>,
    prev_totals: &mut Option<NetworkTotals>,
    refresh_wireless: bool,
) -> Result<NetworkSnapshot, String> {
    let effective_iface = iface_filter
        .map(str::to_string)
        .or_else(default_route_iface);
    let iface_for_counters = effective_iface.as_deref().or(iface_filter);
    let (down_total, up_total) = read_totals(iface_for_counters)?;
    let now = Instant::now();

    let mut down_bps = 0_u64;
    let mut up_bps = 0_u64;
    let mut state = "idle".to_string();

    if let Some(prev) = *prev_totals {
        let elapsed = now
            .saturating_duration_since(prev.collected_at)
            .as_secs_f64();
        if elapsed > 0.0 {
            let down_delta = down_total.saturating_sub(prev.down_total_bytes);
            let up_delta = up_total.saturating_sub(prev.up_total_bytes);
            down_bps = (down_delta as f64 / elapsed).round() as u64;
            up_bps = (up_delta as f64 / elapsed).round() as u64;
            if down_bps > 0 || up_bps > 0 {
                state = "active".to_string();
            }
        }
    }

    *prev_totals = Some(NetworkTotals {
        down_total_bytes: down_total,
        up_total_bytes: up_total,
        collected_at: now,
    });

    let mut snapshot = NetworkSnapshot {
        iface: effective_iface
            .as_deref()
            .or(iface_filter)
            .unwrap_or("all")
            .to_string(),
        down_bps: down_bps.to_string(),
        up_bps: up_bps.to_string(),
        down_kibps: (down_bps / 1024).to_string(),
        up_kibps: (up_bps / 1024).to_string(),
        state,
        down_total_bytes: down_total.to_string(),
        up_total_bytes: up_total.to_string(),
        wireless: None,
    };

    if refresh_wireless {
        if let Some((nm_iface, ssid, strength)) = nm_ssid_strength(iface_for_counters) {
            snapshot.iface = nm_iface.clone();
            let signal_dbm = wireless_iface_stats(Some(&nm_iface))
                .ok()
                .flatten()
                .map(|(_, _, dbm)| format!("{:.0}", dbm))
                .unwrap_or_default();
            snapshot.wireless = Some(WirelessSnapshot {
                iface_override: Some(nm_iface),
                ssid,
                strength_percent: strength.to_string(),
                signal_dbm,
            });
        } else if let Some((wifi_iface, quality, signal_dbm)) =
            wireless_iface_stats(iface_for_counters)?
        {
            let fallback_strength = ((quality / 70.0) * 100.0).clamp(0.0, 100.0) as u8;
            let ssid = read_ssid(&wifi_iface).unwrap_or_else(|| wifi_iface.clone());
            snapshot.wireless = Some(WirelessSnapshot {
                iface_override: None,
                ssid,
                strength_percent: fallback_strength.to_string(),
                signal_dbm: format!("{:.0}", signal_dbm),
            });
        } else {
            snapshot.wireless = Some(WirelessSnapshot {
                iface_override: None,
                ssid: String::new(),
                strength_percent: "0".to_string(),
                signal_dbm: String::new(),
            });
        }
    }

    Ok(snapshot)
}

pub(super) fn apply_snapshot(bus: &SignalBus, snapshot: &NetworkSnapshot) {
    bus.batch(|| {
        bus.set("data.network.iface", &snapshot.iface);
        bus.set("data.network.down_bps", &snapshot.down_bps);
        bus.set("data.network.up_bps", &snapshot.up_bps);
        bus.set("data.network.down_kibps", &snapshot.down_kibps);
        bus.set("data.network.up_kibps", &snapshot.up_kibps);
        bus.set("data.network.state", &snapshot.state);
        bus.set("data.network.down_total_bytes", &snapshot.down_total_bytes);
        bus.set("data.network.up_total_bytes", &snapshot.up_total_bytes);

        if let Some(wifi) = &snapshot.wireless {
            if let Some(iface) = &wifi.iface_override {
                bus.set("data.network.iface", iface);
            }
            bus.set("data.network.ssid", &wifi.ssid);
            bus.set("data.network.wifi_strength_percent", &wifi.strength_percent);
            bus.set("data.network.wifi_signal_dbm", &wifi.signal_dbm);
        }
    });
}
