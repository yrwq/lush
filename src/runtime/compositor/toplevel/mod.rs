mod dispatch;
mod state;

use async_channel::{Receiver, Sender};
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};
use std::thread;
use wayland_client::Connection;
use wayland_client::globals::registry_queue_init;

use crate::protocols::ext_foreign_toplevel_list::ext_foreign_toplevel_list_v1::ext_foreign_toplevel_list_v1;

use self::state::ToplevelState;
use super::ToplevelEntry;

struct ToplevelService {
    listeners: Arc<Mutex<Vec<Sender<Vec<ToplevelEntry>>>>>,
    latest: Arc<Mutex<Vec<ToplevelEntry>>>,
}

static TOPLEVEL_SERVICE: OnceCell<Arc<ToplevelService>> = OnceCell::new();

pub fn subscribe(_output_selector: Option<&str>) -> Option<Receiver<Vec<ToplevelEntry>>> {
    let service = TOPLEVEL_SERVICE
        .get_or_try_init(ToplevelService::start)
        .ok()?
        .clone();

    let (tx, rx) = async_channel::unbounded();
    if let Ok(mut listeners) = service.listeners.lock() {
        listeners.push(tx.clone());
    }

    if let Ok(latest) = service.latest.lock() {
        let _ = tx.try_send(latest.clone());
    }

    Some(rx)
}

impl ToplevelService {
    fn start() -> Result<Arc<Self>, ()> {
        let listeners: Arc<Mutex<Vec<Sender<Vec<ToplevelEntry>>>>> =
            Arc::new(Mutex::new(Vec::new()));
        let latest: Arc<Mutex<Vec<ToplevelEntry>>> = Arc::new(Mutex::new(Vec::new()));

        let listeners_for_thread = listeners.clone();
        let latest_for_thread = latest.clone();

        thread::Builder::new()
            .name("toplevel-listener".to_string())
            .spawn(move || run_toplevel_listener(listeners_for_thread, latest_for_thread))
            .map_err(|_| ())?;

        Ok(Arc::new(Self { listeners, latest }))
    }
}

fn run_toplevel_listener(
    listeners: Arc<Mutex<Vec<Sender<Vec<ToplevelEntry>>>>>,
    latest: Arc<Mutex<Vec<ToplevelEntry>>>,
) {
    let conn = match Connection::connect_to_env() {
        Ok(conn) => conn,
        Err(err) => {
            log::debug!("toplevel-list: wayland connect failed: {}", err);
            return;
        }
    };

    let (globals, mut event_queue) = match registry_queue_init::<ToplevelState>(&conn) {
        Ok(v) => v,
        Err(err) => {
            log::debug!("toplevel-list: registry init failed: {}", err);
            return;
        }
    };

    let qh = event_queue.handle();
    let mut state = ToplevelState::new(listeners, latest);
    let manager: ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1 =
        match globals.bind(&qh, 1..=1, ()) {
            Ok(v) => v,
            Err(err) => {
                log::debug!("toplevel-list: protocol unavailable: {}", err);
                return;
            }
        };
    state.manager = Some(manager);

    loop {
        if event_queue.blocking_dispatch(&mut state).is_err() {
            break;
        }
    }
}
