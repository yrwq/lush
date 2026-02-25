use std::ffi::CString;
use std::path::Path;
use std::time::Duration;

use gio::prelude::*;
use std::sync::mpsc;

use super::evented::EventedProvider;
use crate::runtime::signal_bus::SignalBus;

fn gb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0 / 1024.0
}

pub struct DiskWatcher {
    _evented: EventedProvider,
    _fs_monitor: Option<gio::FileMonitor>,
}

#[derive(Clone, Debug)]
struct DiskSnapshot {
    path: String,
    total_percent: String,
    used_percent: String,
    free_percent: String,
    total_gb: String,
    used_gb: String,
    free_gb: String,
    total_bytes: String,
    used_bytes: String,
    free_bytes: String,
}

fn collect_snapshot(path: Option<&str>) -> Result<DiskSnapshot, String> {
    let path = path.unwrap_or("/");
    let c_path = CString::new(path).map_err(|_| format!("invalid disk path '{}'", path))?;

    let mut vfs = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    let rc = unsafe { libc::statvfs(c_path.as_ptr(), vfs.as_mut_ptr()) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let vfs = unsafe { vfs.assume_init() };

    let block_size = if vfs.f_frsize > 0 {
        vfs.f_frsize
    } else {
        vfs.f_bsize
    };
    if block_size == 0 {
        return Ok(DiskSnapshot {
            path: path.to_string(),
            total_percent: "100".to_string(),
            used_percent: "0.0".to_string(),
            free_percent: "0.0".to_string(),
            total_gb: "0.0".to_string(),
            used_gb: "0.0".to_string(),
            free_gb: "0.0".to_string(),
            total_bytes: "0".to_string(),
            used_bytes: "0".to_string(),
            free_bytes: "0".to_string(),
        });
    }

    let total_bytes = vfs.f_blocks.saturating_mul(block_size);
    let free_bytes = vfs.f_bavail.saturating_mul(block_size);
    let used_bytes = total_bytes.saturating_sub(free_bytes);
    if total_bytes == 0 {
        return Ok(DiskSnapshot {
            path: path.to_string(),
            total_percent: "100".to_string(),
            used_percent: "0.0".to_string(),
            free_percent: "0.0".to_string(),
            total_gb: "0.0".to_string(),
            used_gb: "0.0".to_string(),
            free_gb: "0.0".to_string(),
            total_bytes: "0".to_string(),
            used_bytes: "0".to_string(),
            free_bytes: "0".to_string(),
        });
    }

    let used_percent = ((used_bytes as f64 / total_bytes as f64) * 100.0).clamp(0.0, 100.0);
    let free_percent = (100.0 - used_percent).clamp(0.0, 100.0);

    Ok(DiskSnapshot {
        path: path.to_string(),
        total_percent: "100".to_string(),
        used_percent: format!("{:.1}", used_percent),
        free_percent: format!("{:.1}", free_percent),
        total_gb: format!("{:.1}", gb(total_bytes)),
        used_gb: format!("{:.1}", gb(used_bytes)),
        free_gb: format!("{:.1}", gb(free_bytes)),
        total_bytes: total_bytes.to_string(),
        used_bytes: used_bytes.to_string(),
        free_bytes: free_bytes.to_string(),
    })
}

fn apply_snapshot(bus: &SignalBus, snapshot: &DiskSnapshot) {
    bus.batch(|| {
        bus.set("data.disk.path", &snapshot.path);
        bus.set("data.disk.total_percent", &snapshot.total_percent);
        bus.set("data.disk.used_percent", &snapshot.used_percent);
        bus.set("data.disk.free_percent", &snapshot.free_percent);
        bus.set("data.disk.total_gb", &snapshot.total_gb);
        bus.set("data.disk.used_gb", &snapshot.used_gb);
        bus.set("data.disk.free_gb", &snapshot.free_gb);
        bus.set("data.disk.total_bytes", &snapshot.total_bytes);
        bus.set("data.disk.used_bytes", &snapshot.used_bytes);
        bus.set("data.disk.free_bytes", &snapshot.free_bytes);
    });
}

pub fn start_evented(
    bus: SignalBus,
    path: Option<String>,
    interval_seconds: u32,
) -> Result<DiskWatcher, String> {
    let interval = interval_seconds.max(1);
    let interval_duration = Duration::from_secs(interval as u64);
    let (snapshot_tx, snapshot_rx) = async_channel::unbounded::<DiskSnapshot>();
    let bus_for_snapshots = bus.clone();
    super::evented::spawn_snapshot_drain(snapshot_rx, move |snapshot| {
        apply_snapshot(&bus_for_snapshots, &snapshot);
    });

    let (trigger_tx, trigger_rx) = super::evented::trigger_channel();
    let mut evented = EventedProvider::new();
    let stop = evented.stop_flag();
    let path_for_collector = path.clone();
    let collector_thread = super::evented::spawn_periodic_collector(
        "data.disk",
        stop.clone(),
        trigger_rx,
        interval_duration,
        snapshot_tx,
        move |_reason| collect_snapshot(path_for_collector.as_deref()),
    );
    let event_thread = super::evented::spawn_udev_trigger_thread(stop, "block", trigger_tx.clone());
    evented.push_thread(collector_thread);
    evented.push_thread(event_thread);

    let fs_monitor = create_fs_monitor(path.as_deref(), trigger_tx);
    Ok(DiskWatcher {
        _evented: evented,
        _fs_monitor: fs_monitor,
    })
}

fn create_fs_monitor(path: Option<&str>, tx: mpsc::SyncSender<()>) -> Option<gio::FileMonitor> {
    let requested = path.unwrap_or("/");
    let requested_path = Path::new(requested);
    let watch_path = if requested_path.is_dir() {
        requested_path
    } else {
        requested_path.parent().unwrap_or(requested_path)
    };

    let file = gio::File::for_path(watch_path);
    let monitor = match file.monitor_directory(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
    {
        Ok(m) => m,
        Err(err) => {
            log::debug!(
                "data.disk fs monitor unavailable for '{}': {}",
                watch_path.display(),
                err
            );
            return None;
        }
    };

    monitor.connect_changed(move |_, _, _, event| {
        use gio::FileMonitorEvent as E;
        if matches!(
            event,
            E::Created
                | E::Deleted
                | E::Moved
                | E::Renamed
                | E::ChangesDoneHint
                | E::AttributeChanged
        ) {
            super::evented::trigger_now(&tx);
        }
    });

    Some(monitor)
}
