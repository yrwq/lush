use async_channel::Receiver;
use once_cell::sync::OnceCell;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::broadcast::BroadcastHub;
use super::common::{self, EventLoopStep};
use crate::runtime::compositor::{
    CompositorSnapshot, CompositorStateSnapshot, FocusedWindowSnapshot, ToplevelEntry,
};

pub fn available() -> bool {
    query_json("monitors").is_some()
}

pub fn focus_workspace(index_1_based: u32) -> Result<(), String> {
    let reply = send_command(&format!("dispatch workspace {}", index_1_based))
        .ok_or_else(|| "failed to send hyprland workspace dispatch".to_string())?;
    if reply.trim().eq_ignore_ascii_case("ok") {
        Ok(())
    } else {
        Err(format!(
            "hyprland workspace dispatch failed: {}",
            reply.trim()
        ))
    }
}

static STATE_SERVICES: OnceCell<
    Mutex<HashMap<String, Arc<BroadcastHub<CompositorStateSnapshot>>>>,
> = OnceCell::new();

pub fn subscribe_state(output_selector: Option<&str>) -> Option<Receiver<CompositorStateSnapshot>> {
    if !available() {
        return None;
    }

    let (key, selector) = common::selector_key_and_filter(output_selector);
    let service = state_service_for_key(&key, selector)?;
    service.subscribe()
}

fn state_service_for_key(
    key: &str,
    selector: Option<String>,
) -> Option<Arc<BroadcastHub<CompositorStateSnapshot>>> {
    let services = STATE_SERVICES.get_or_init(|| Mutex::new(HashMap::new()));
    common::state_service_for_key(services, key, move |service| {
        thread::Builder::new()
            .name("hyprland-state-listener".to_string())
            .spawn(move || {
                let selector = selector.as_deref();
                loop {
                    let Some(stream) = connect_listener_socket() else {
                        thread::sleep(Duration::from_millis(800));
                        if let Some(snapshot) = collect_state(selector) {
                            service.publish(snapshot);
                        }
                        continue;
                    };

                    let mut reader = BufReader::new(stream);
                    let mut line = String::new();
                    common::run_refresh_loop(
                        &service,
                        || collect_state(selector),
                        || {
                            line.clear();
                            match reader.read_line(&mut line) {
                                Ok(0) => EventLoopStep::Break,
                                Ok(_) => {
                                    if should_refresh_for_event(&line) {
                                        EventLoopStep::Refresh
                                    } else {
                                        EventLoopStep::Continue
                                    }
                                }
                                Err(_) => EventLoopStep::Break,
                            }
                        },
                    );

                    thread::sleep(Duration::from_millis(120));
                }
            })
            .ok()?;
        Some(())
    })
}

fn collect_state(output_selector: Option<&str>) -> Option<CompositorStateSnapshot> {
    let monitors = query_json("monitors")?;
    let workspaces = query_json("workspaces")?;
    let clients = query_json("clients")?;
    let active_client = query_json("activewindow");
    let active_workspace = query_json("activeworkspace");
    let active_client_address = active_client
        .as_ref()
        .and_then(|v| v.get("address"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let selected_monitor =
        resolve_selected_monitor_json(output_selector, &monitors, active_workspace.as_ref());
    let selected_monitor_name = selected_monitor.as_deref();

    let mut focused_mask = 0_u32;
    let mut occupied_mask = 0_u32;
    let urgent_mask = 0_u32;
    let mut focused_window = FocusedWindowSnapshot::default();
    let mut toplevels = Vec::new();

    if let Some(items) = workspaces.as_array() {
        for ws in items {
            let id = ws.get("id").and_then(Value::as_i64).unwrap_or_default() as u32;
            if !(1..=32).contains(&id) {
                continue;
            }
            if let Some(monitor_name) = selected_monitor_name {
                let ws_monitor = ws.get("monitor").and_then(Value::as_str).unwrap_or("");
                if !ws_monitor.eq_ignore_ascii_case(monitor_name) {
                    continue;
                }
            }
            occupied_mask |= 1_u32 << (id - 1);
        }
    }

    if let Some(items) = monitors.as_array() {
        if let Some(selected) = selected_monitor_name {
            if let Some(monitor) = items.iter().find(|m| {
                m.get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| name.eq_ignore_ascii_case(selected))
            }) {
                let id = monitor
                    .get("activeWorkspace")
                    .and_then(|v| v.get("id"))
                    .and_then(Value::as_i64)
                    .unwrap_or_default() as u32;
                if (1..=32).contains(&id) {
                    focused_mask |= 1_u32 << (id - 1);
                }
            }
        } else {
            for monitor in items {
                let id = monitor
                    .get("activeWorkspace")
                    .and_then(|v| v.get("id"))
                    .and_then(Value::as_i64)
                    .unwrap_or_default() as u32;
                if !(1..=32).contains(&id) {
                    continue;
                }
                if monitor.get("focused").and_then(Value::as_bool) == Some(true) {
                    focused_mask |= 1_u32 << (id - 1);
                }
            }
        }
    }

    if focused_mask == 0
        && let Some(id) = active_workspace
            .as_ref()
            .and_then(|v| v.get("id"))
            .and_then(Value::as_i64)
            .map(|v| v as u32)
            .filter(|id| (1..=32).contains(id))
    {
        focused_mask |= 1_u32 << (id - 1);
    }
    let focused_workspace_id = first_set_bit(focused_mask);

    let monitor_names_by_id = monitor_names_by_id_json(&monitors);
    if let Some(items) = clients.as_array() {
        for client in items {
            if client.get("mapped").and_then(Value::as_bool) == Some(false) {
                continue;
            }
            let monitor_id = client
                .get("monitor")
                .and_then(Value::as_i64)
                .unwrap_or_default() as i128;
            if !selected_monitor_matches_json(
                monitor_id,
                selected_monitor_name,
                &monitor_names_by_id,
            ) {
                continue;
            }

            let workspace_id = client
                .get("workspace")
                .and_then(|v| v.get("id"))
                .and_then(Value::as_i64)
                .and_then(|id| (id > 0).then_some(id as u32));
            let workspace_mask = workspace_id
                .and_then(|id| (1..=32).contains(&id).then_some(1_u32 << (id - 1)))
                .unwrap_or(0);
            let title = first_nonempty(client.get("title"), client.get("initialTitle"));
            let app_id = first_nonempty(client.get("class"), client.get("initialClass"));
            let identifier = client
                .get("address")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let focus_candidate = active_client_address.as_deref() == Some(identifier.as_str())
                || client.get("focusHistoryID").and_then(Value::as_i64) == Some(0);
            let focused =
                focus_candidate && workspace_id.is_some() && workspace_id == focused_workspace_id;
            if focused {
                focused_window.title = title.clone();
                focused_window.app_id = app_id.clone();
                focused_window.identifier = identifier.clone();
            }
            toplevels.push(ToplevelEntry {
                title,
                app_id,
                identifier,
                workspace_id,
                workspace_mask,
                focused,
            });
        }
    }

    Some(CompositorStateSnapshot {
        workspace: CompositorSnapshot {
            focused_mask,
            occupied_mask,
            urgent_mask,
            focused_window,
        },
        toplevels,
    })
}

fn should_refresh_for_event(line: &str) -> bool {
    let event = line.split_once(">>").map(|(name, _)| name).unwrap_or(line);
    matches!(
        event.trim(),
        "workspace"
            | "workspacev2"
            | "focusedmon"
            | "focusedmonv2"
            | "activewindow"
            | "activewindowv2"
            | "openwindow"
            | "openwindowv2"
            | "closewindow"
            | "closewindowv2"
            | "movewindow"
            | "movewindowv2"
            | "windowtitle"
            | "windowtitlev2"
            | "renameworkspace"
            | "renameworkspacev2"
            | "createworkspace"
            | "createworkspacev2"
            | "destroyworkspace"
            | "destroyworkspacev2"
    )
}

fn resolve_selected_monitor_json(
    output_selector: Option<&str>,
    monitors: &Value,
    active_workspace: Option<&Value>,
) -> Option<String> {
    let selector = common::normalize_selector(output_selector);
    if selector.is_empty() {
        return None;
    }
    let items = monitors.as_array()?;
    if common::selector_is_focused_or_empty(&selector) {
        return items
            .iter()
            .find(|m| m.get("focused").and_then(Value::as_bool) == Some(true))
            .and_then(|m| m.get("name"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                active_workspace
                    .and_then(|v| v.get("monitor"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });
    }
    if let Some(index) = common::selector_index(&selector) {
        let mut names: Vec<String> = items
            .iter()
            .filter_map(|m| m.get("name").and_then(Value::as_str))
            .map(str::to_string)
            .collect();
        names.sort();
        return names.get(index).cloned();
    }
    Some(selector)
}

fn monitor_names_by_id_json(monitors: &Value) -> HashMap<i128, String> {
    let mut out = HashMap::new();
    let Some(items) = monitors.as_array() else {
        return out;
    };
    for item in items {
        let Some(id) = item.get("id").and_then(Value::as_i64) else {
            continue;
        };
        let Some(name) = item.get("name").and_then(Value::as_str) else {
            continue;
        };
        out.insert(id as i128, name.to_string());
    }
    out
}

fn selected_monitor_matches_json(
    monitor_id: i128,
    selected_monitor_name: Option<&str>,
    monitor_names_by_id: &HashMap<i128, String>,
) -> bool {
    let Some(selected) = selected_monitor_name else {
        return true;
    };
    monitor_names_by_id
        .get(&monitor_id)
        .map(|m| m.eq_ignore_ascii_case(selected))
        .unwrap_or(false)
}

fn connect_listener_socket() -> Option<UnixStream> {
    for path in hypr_socket_candidates(".socket2.sock") {
        if let Ok(stream) = UnixStream::connect(&path) {
            return Some(stream);
        }
    }
    None
}

fn connect_command_socket() -> Option<UnixStream> {
    for path in hypr_socket_candidates(".socket.sock") {
        if let Ok(stream) = UnixStream::connect(&path) {
            return Some(stream);
        }
    }
    None
}

fn hypr_socket_candidates(socket_name: &str) -> Vec<String> {
    let Some(sig) = hypr_instance_signature() else {
        return Vec::new();
    };
    let mut out = vec![format!("/tmp/hypr/{sig}/{socket_name}")];
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR")
        && !runtime.trim().is_empty()
    {
        out.push(format!("{runtime}/hypr/{sig}/{socket_name}"));
    }
    out
}

fn hypr_instance_signature() -> Option<String> {
    std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn query_json(query: &str) -> Option<Value> {
    let raw = send_json_query(query)?;
    serde_json::from_slice::<Value>(&raw).ok()
}

fn send_json_query(query: &str) -> Option<Vec<u8>> {
    let mut stream = connect_command_socket()?;
    let payload = format!("j/{query}");
    stream.write_all(payload.as_bytes()).ok()?;

    let mut out = Vec::new();
    stream.read_to_end(&mut out).ok()?;
    if out.is_empty() {
        return None;
    }
    Some(out)
}

fn send_command(command: &str) -> Option<String> {
    let mut stream = connect_command_socket()?;
    let payload = format!("/{command}");
    stream.write_all(payload.as_bytes()).ok()?;

    let mut out = Vec::new();
    stream.read_to_end(&mut out).ok()?;
    String::from_utf8(out).ok()
}

fn first_nonempty(primary: Option<&Value>, fallback: Option<&Value>) -> String {
    primary
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            fallback
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
        })
        .unwrap_or_default()
        .to_string()
}

fn first_set_bit(mask: u32) -> Option<u32> {
    if mask == 0 {
        None
    } else {
        Some(mask.trailing_zeros() + 1)
    }
}
