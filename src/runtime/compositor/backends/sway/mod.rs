mod broadcast;
mod ipc;
mod toplevels;
mod tree;
mod workspaces;

use async_channel::Receiver;
use once_cell::sync::OnceCell;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::runtime::compositor::CompositorStateSnapshot;

use self::broadcast::BroadcastHub;

pub fn available() -> bool {
    let Some(path) = ipc::socket_path() else {
        return false;
    };
    std::os::unix::net::UnixStream::connect(path).is_ok()
}

pub fn focus_workspace(index_1_based: u32) -> Result<(), String> {
    ipc::run_command(&format!("workspace number {}", index_1_based))
}

static STATE_SERVICES: OnceCell<
    Mutex<HashMap<String, Arc<BroadcastHub<CompositorStateSnapshot>>>>,
> = OnceCell::new();

pub fn subscribe_state(output_selector: Option<&str>) -> Option<Receiver<CompositorStateSnapshot>> {
    if !available() {
        return None;
    }

    let (key, selector) = selector_key_and_filter(output_selector);
    let service = state_service_for_key(&key, selector)?;
    service.subscribe()
}

fn state_service_for_key(
    key: &str,
    selector: Option<String>,
) -> Option<Arc<BroadcastHub<CompositorStateSnapshot>>> {
    let services = STATE_SERVICES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut services = services.lock().ok()?;
    if let Some(existing) = services.get(key) {
        return Some(existing.clone());
    }

    let service = Arc::new(BroadcastHub::new());
    let service_for_thread = service.clone();
    thread::Builder::new()
        .name("sway-state-listener".to_string())
        .spawn(move || {
            let mut event_stream = match ipc::subscribe_events(["workspace", "window"]) {
                Some(v) => v,
                None => return,
            };
            let selector = selector.as_deref();

            if let Some(snapshot) = collect_state(selector) {
                service_for_thread.publish(snapshot);
            }

            loop {
                if ipc::read_message(&mut event_stream).is_err() {
                    break;
                }
                let Some(snapshot) = collect_state(selector) else {
                    continue;
                };
                service_for_thread.publish(snapshot);
            }
        })
        .ok()?;

    services.insert(key.to_string(), service.clone());
    Some(service)
}

fn collect_state(output_selector: Option<&str>) -> Option<CompositorStateSnapshot> {
    let workspace = workspaces::collect_snapshot(output_selector)?;
    let toplevels = toplevels::collect_toplevels(output_selector)?;
    Some(CompositorStateSnapshot {
        workspace,
        toplevels,
    })
}

pub(super) fn selector_key_and_filter(output_selector: Option<&str>) -> (String, Option<String>) {
    let key = output_selector
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("")
        .to_string();
    let filter = (!key.is_empty()).then_some(key.clone());
    (key, filter)
}

pub(super) fn resolve_selected_output(selector: &str, workspaces: &Value) -> Option<String> {
    let selector = selector.trim();
    if selector.is_empty() || selector.eq_ignore_ascii_case("focused") {
        return focused_output_name(workspaces);
    }
    if let Ok(index) = selector.parse::<usize>() {
        return ipc::output_name_by_index(index);
    }
    Some(selector.to_string())
}

pub(super) fn focused_output_name(workspaces: &Value) -> Option<String> {
    workspaces
        .as_array()?
        .iter()
        .find(|ws| ws.get("focused").and_then(Value::as_bool) == Some(true))
        .and_then(|ws| ws.get("output"))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}
