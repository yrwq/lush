mod snapshot;
mod sys;
mod wireless;

use std::time::Duration;

use super::evented::{CollectReason, EventedProvider};
use super::{NetworkTotals, NetworkTotalsState};
use crate::runtime::signal_bus::SignalBus;

use self::snapshot::{NetworkSnapshot, apply_snapshot, collect_snapshot};

pub struct NetworkWatcher {
    _evented: EventedProvider,
}

pub fn start_evented(
    bus: SignalBus,
    iface_filter: Option<String>,
    _network_prev_totals: NetworkTotalsState,
    interval_seconds: u32,
) -> Result<NetworkWatcher, String> {
    let interval = interval_seconds.max(1);
    let interval_duration = Duration::from_secs(interval as u64);

    let (snapshot_tx, snapshot_rx) = async_channel::unbounded::<NetworkSnapshot>();
    let bus_for_snapshots = bus.clone();
    super::evented::spawn_snapshot_drain(snapshot_rx, move |snapshot| {
        apply_snapshot(&bus_for_snapshots, &snapshot);
    });

    let (trigger_tx, trigger_rx) = super::evented::trigger_channel();
    let mut evented = EventedProvider::new();
    let stop = evented.stop_flag();
    let iface_for_collector = iface_filter.clone();
    let mut prev_totals: Option<NetworkTotals> = None;
    let mut periodic_tick: u32 = 0;
    let collector_thread = super::evented::spawn_periodic_collector(
        "data.network",
        stop.clone(),
        trigger_rx,
        interval_duration,
        snapshot_tx,
        move |reason| {
            let refresh_wireless = match reason {
                CollectReason::Initial => true,
                CollectReason::Event => true,
                CollectReason::Interval => {
                    periodic_tick = periodic_tick.saturating_add(1);
                    periodic_tick.is_multiple_of(15)
                }
            };
            collect_snapshot(
                iface_for_collector.as_deref(),
                &mut prev_totals,
                refresh_wireless,
            )
        },
    );
    let event_thread = super::evented::spawn_udev_trigger_thread(stop, "net", trigger_tx);
    evented.push_thread(collector_thread);
    evented.push_thread(event_thread);

    Ok(NetworkWatcher { _evented: evented })
}
