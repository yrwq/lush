mod dispatch;
mod model;
mod state;

use std::sync::{Arc, Mutex};

use async_channel::Receiver;
use once_cell::sync::OnceCell;
use wayland_client::globals::registry_queue_init;
use wayland_client::{Connection, Proxy};

use crate::protocols::river_status::river_status_unstable_v1::zriver_status_manager_v1;

pub use model::RiverSnapshot;
use model::{
    LatestSnapshots, LatestSnapshotsStore, SnapshotListener, SnapshotListeners,
    snapshot_from_latest,
};
use state::RiverState;

struct RiverService {
    listeners: SnapshotListeners,
    latest: LatestSnapshotsStore,
}

static RIVER_SERVICE: OnceCell<Arc<RiverService>> = OnceCell::new();

pub fn subscribe(selector: Option<&str>) -> Option<Receiver<RiverSnapshot>> {
    if !river_status_available() {
        return None;
    }

    let service = RIVER_SERVICE
        .get_or_try_init(RiverService::start)
        .ok()?
        .clone();

    let (tx, rx) = async_channel::unbounded();
    let mut listeners = service.listeners.lock().ok()?;
    let selector_owned = selector.map(|s| s.to_string());
    listeners.push(SnapshotListener {
        selector: selector_owned.clone(),
        tx: tx.clone(),
        last_sent: None,
    });

    if let Ok(latest) = service.latest.lock() {
        let snapshot = snapshot_from_latest(&latest, selector_owned.as_deref());
        let _ = tx.try_send(snapshot);
    }
    Some(rx)
}

pub fn focused_output_name() -> Option<String> {
    let service = RIVER_SERVICE
        .get_or_try_init(RiverService::start)
        .ok()?
        .clone();
    let latest = service.latest.lock().ok()?;
    latest.focused_output_name.clone()
}

pub fn river_status_available() -> bool {
    let conn = match Connection::connect_to_env() {
        Ok(conn) => conn,
        Err(_) => return false,
    };

    let (globals, _queue) = match registry_queue_init::<RiverState>(&conn) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let manager_iface = zriver_status_manager_v1::ZriverStatusManagerV1::interface().name;
    globals
        .contents()
        .clone_list()
        .iter()
        .any(|g| g.interface == manager_iface)
}

impl RiverService {
    fn start() -> Result<Arc<Self>, ()> {
        let listeners: SnapshotListeners = Arc::new(Mutex::new(Vec::new()));
        let latest: LatestSnapshotsStore = Arc::new(Mutex::new(LatestSnapshots::default()));
        let listeners_for_thread = listeners.clone();
        let latest_for_thread = latest.clone();

        std::thread::Builder::new()
            .name("river-status-listener".to_string())
            .spawn(move || run(listeners_for_thread, latest_for_thread))
            .map_err(|_| ())?;

        Ok(Arc::new(Self { listeners, latest }))
    }
}

fn run(listeners: SnapshotListeners, latest: LatestSnapshotsStore) {
    let conn = match Connection::connect_to_env() {
        Ok(conn) => conn,
        Err(err) => {
            log::warn!("river-status: wayland connect failed: {}", err);
            return;
        }
    };

    let (globals, mut event_queue) = match registry_queue_init::<RiverState>(&conn) {
        Ok(v) => v,
        Err(err) => {
            log::warn!("river-status: registry init failed: {}", err);
            return;
        }
    };

    let qh = event_queue.handle();
    let mut state = RiverState::new(listeners, latest);
    if let Err(err) = state.bind_initial(&globals, &qh) {
        log::warn!("river-status: bind failed: {}", err);
        return;
    }

    loop {
        if event_queue.blocking_dispatch(&mut state).is_err() {
            break;
        }
    }
}
