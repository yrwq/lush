use once_cell::sync::OnceCell;
use std::sync::mpsc::{self, Receiver, Sender};

use wayland_client::globals::{GlobalList, registry_queue_init};
use wayland_client::protocol::wl_seat;
use wayland_client::{Connection, EventQueue, Proxy, QueueHandle};

use crate::protocols::wlr_foreign_toplevel_management::wlr_foreign_toplevel_management_unstable_v1::{
    zwlr_foreign_toplevel_manager_v1,
};

use super::matching::ordered_candidates;
use super::state::WlrActionState;

enum ActionKind {
    Activate,
    Close,
    SetMinimized(bool),
}

struct ActionRequest {
    kind: ActionKind,
    app_id: String,
    title: String,
    duplicate_rank: usize,
    reply_tx: Sender<Result<(), String>>,
}

struct WorkerSession {
    conn: Connection,
    globals: wayland_client::globals::GlobalList,
    event_queue: EventQueue<WlrActionState>,
    qh: QueueHandle<WlrActionState>,
    state: WlrActionState,
    _manager: zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
}

static ACTION_WORKER: OnceCell<Sender<ActionRequest>> = OnceCell::new();

pub(super) fn activate(app_id: &str, title: &str, duplicate_rank: usize) -> Result<(), String> {
    send_request(ActionKind::Activate, app_id, title, duplicate_rank)
}

pub(super) fn close(app_id: &str, title: &str, duplicate_rank: usize) -> Result<(), String> {
    send_request(ActionKind::Close, app_id, title, duplicate_rank)
}

pub(super) fn set_minimized(
    app_id: &str,
    title: &str,
    duplicate_rank: usize,
    minimized: bool,
) -> Result<(), String> {
    send_request(
        ActionKind::SetMinimized(minimized),
        app_id,
        title,
        duplicate_rank,
    )
}

fn send_request(
    kind: ActionKind,
    app_id: &str,
    title: &str,
    duplicate_rank: usize,
) -> Result<(), String> {
    let tx = action_sender()?;
    let (reply_tx, reply_rx) = mpsc::channel();
    tx.send(ActionRequest {
        kind,
        app_id: app_id.to_string(),
        title: title.to_string(),
        duplicate_rank,
        reply_tx,
    })
    .map_err(|_| "wlr action worker is unavailable".to_string())?;

    reply_rx
        .recv()
        .map_err(|_| "wlr action worker disconnected".to_string())?
}

fn action_sender() -> Result<&'static Sender<ActionRequest>, String> {
    ACTION_WORKER.get_or_try_init(|| {
        let (tx, rx) = mpsc::channel::<ActionRequest>();
        std::thread::Builder::new()
            .name("wlr-action-worker".to_string())
            .spawn(move || worker_loop(rx))
            .map_err(|err| format!("failed to start wlr action worker: {}", err))?;
        Ok(tx)
    })
}

fn worker_loop(rx: Receiver<ActionRequest>) {
    let mut session: Option<WorkerSession> = None;

    for request in rx {
        if session.is_none() {
            match create_session() {
                Ok(new_session) => session = Some(new_session),
                Err(err) => {
                    let _ = request.reply_tx.send(Err(err));
                    continue;
                }
            }
        }

        let result = if let Some(current) = session.as_mut() {
            handle_request(current, &request)
        } else {
            Err("wlr action session is unavailable".to_string())
        };

        if result.is_err() {
            session = None;
        }

        let _ = request.reply_tx.send(result);
    }
}

fn create_session() -> Result<WorkerSession, String> {
    let conn = Connection::connect_to_env()
        .map_err(|err| format!("wlr toplevel wayland connect failed: {}", err))?;
    let (globals, event_queue) = registry_queue_init::<WlrActionState>(&conn)
        .map_err(|err| format!("wlr toplevel registry init failed: {}", err))?;

    let qh = event_queue.handle();
    let mut state = WlrActionState::default();
    let manager: zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1 = globals
        .bind(&qh, 1..=3, ())
        .map_err(|err| format!("wlr toplevel protocol unavailable: {}", err))?;

    bind_first_seat_if_available(&globals, &qh, &mut state);

    Ok(WorkerSession {
        conn,
        globals,
        event_queue,
        qh,
        state,
        _manager: manager,
    })
}

fn handle_request(session: &mut WorkerSession, request: &ActionRequest) -> Result<(), String> {
    if !matches!(request.kind, ActionKind::Activate) {
        // refresh seat lazily for non-activate requests too, keeps session state coherent
        bind_first_seat_if_available(&session.globals, &session.qh, &mut session.state);
    }

    for _ in 0..8 {
        session
            .event_queue
            .roundtrip(&mut session.state)
            .map_err(|_| "wlr toplevel roundtrip failed".to_string())?;

        if session.state.seat.is_none() {
            bind_first_seat_if_available(&session.globals, &session.qh, &mut session.state);
        }

        let candidates = ordered_candidates(&session.state, &request.app_id, &request.title);
        if candidates.is_empty() {
            continue;
        }
        let pick = request
            .duplicate_rank
            .min(candidates.len().saturating_sub(1));
        let entry = candidates[pick];

        let Some(handle) = entry.handle.as_ref() else {
            continue;
        };

        match request.kind {
            ActionKind::Activate => {
                let Some(seat) = session.state.seat.as_ref() else {
                    continue;
                };
                handle.activate(seat);
            }
            ActionKind::Close => handle.close(),
            ActionKind::SetMinimized(minimized) => {
                if minimized {
                    handle.set_minimized();
                } else {
                    handle.unset_minimized();
                }
            }
        }

        let _ = session.conn.flush();
        let _ = session.event_queue.roundtrip(&mut session.state);
        return Ok(());
    }

    match request.kind {
        ActionKind::Activate => Err(format!(
            "wlr activate target not found or no wl_seat for app_id='{}' title='{}'",
            request.app_id, request.title
        )),
        _ => Err(format!(
            "wlr toplevel entry not found for app_id='{}' title='{}'",
            request.app_id, request.title
        )),
    }
}

fn bind_first_seat_if_available(
    globals: &GlobalList,
    qh: &QueueHandle<WlrActionState>,
    state: &mut WlrActionState,
) {
    if state.seat.is_some() {
        return;
    }
    let snapshot = globals.contents().clone_list();
    let registry = globals.registry();
    if let Some(seat_global) = snapshot
        .iter()
        .find(|g| g.interface == wl_seat::WlSeat::interface().name)
    {
        state.seat = Some(registry.bind(seat_global.name, seat_global.version.min(3), qh, ()));
    }
}
