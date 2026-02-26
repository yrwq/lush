use async_channel::Receiver;
use async_channel::Sender;
use once_cell::sync::OnceCell;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use super::ipc::{
    SWAY_IPC_GET_TREE, SWAY_IPC_GET_WORKSPACES, read_message, request_json, subscribe_events,
};
use super::tree::{node_app_id, node_title};
use crate::runtime::compositor::ToplevelEntry;

struct ToplevelService {
    listeners: Arc<Mutex<Vec<Sender<Vec<ToplevelEntry>>>>>,
    latest: Arc<Mutex<Option<Vec<ToplevelEntry>>>>,
}

static TOPLEVEL_SERVICES: OnceCell<Mutex<HashMap<String, Arc<ToplevelService>>>> = OnceCell::new();

pub(super) fn subscribe(output_selector: Option<&str>) -> Option<Receiver<Vec<ToplevelEntry>>> {
    if !super::available() {
        return None;
    }

    let key = output_selector
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("")
        .to_string();
    let selector = (!key.is_empty()).then_some(key.clone());
    let service = toplevel_service_for_key(&key, selector)?;

    let (tx, rx) = async_channel::unbounded();
    if let Ok(mut listeners) = service.listeners.lock() {
        listeners.push(tx.clone());
    }
    if let Ok(latest) = service.latest.lock()
        && let Some(snapshot) = latest.clone()
    {
        let _ = tx.try_send(snapshot);
    }

    Some(rx)
}

fn toplevel_service_for_key(key: &str, selector: Option<String>) -> Option<Arc<ToplevelService>> {
    let services = TOPLEVEL_SERVICES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut services = services.lock().ok()?;
    if let Some(existing) = services.get(key) {
        return Some(existing.clone());
    }

    let created = ToplevelService::start(selector)?;
    services.insert(key.to_string(), created.clone());
    Some(created)
}

impl ToplevelService {
    fn start(selector: Option<String>) -> Option<Arc<Self>> {
        let listeners: Arc<Mutex<Vec<Sender<Vec<ToplevelEntry>>>>> =
            Arc::new(Mutex::new(Vec::new()));
        let latest: Arc<Mutex<Option<Vec<ToplevelEntry>>>> = Arc::new(Mutex::new(None));
        let listeners_for_thread = listeners.clone();
        let latest_for_thread = latest.clone();

        thread::Builder::new()
            .name("sway-toplevels-listener".to_string())
            .spawn(move || {
                let mut event_stream = match subscribe_events(["workspace", "window"]) {
                    Some(v) => v,
                    None => return,
                };
                let selector = selector.as_deref();

                if let Some(snapshot) = collect_toplevels(selector) {
                    publish_snapshot(&listeners_for_thread, &latest_for_thread, snapshot);
                }

                loop {
                    if read_message(&mut event_stream).is_err() {
                        break;
                    }

                    let Some(snapshot) = collect_toplevels(selector) else {
                        continue;
                    };

                    publish_snapshot(&listeners_for_thread, &latest_for_thread, snapshot);
                }
            })
            .ok()?;

        Some(Arc::new(Self { listeners, latest }))
    }
}

fn publish_snapshot(
    listeners: &Arc<Mutex<Vec<Sender<Vec<ToplevelEntry>>>>>,
    latest: &Arc<Mutex<Option<Vec<ToplevelEntry>>>>,
    snapshot: Vec<ToplevelEntry>,
) {
    if let Ok(mut stored) = latest.lock() {
        if stored.as_ref() == Some(&snapshot) {
            return;
        }
        *stored = Some(snapshot.clone());
    }

    if let Ok(mut refs) = listeners.lock() {
        refs.retain(|tx| tx.try_send(snapshot.clone()).is_ok());
    }
}

fn collect_toplevels(output_selector: Option<&str>) -> Option<Vec<ToplevelEntry>> {
    let tree = request_json(SWAY_IPC_GET_TREE, "")?;
    let workspaces = request_json(SWAY_IPC_GET_WORKSPACES, "")?;

    let selected_output = match output_selector {
        Some(selector) => resolve_selected_output(selector.trim(), &workspaces),
        None => None,
    };

    let mut out = Vec::<ToplevelEntry>::new();
    for output in tree.get("nodes").and_then(Value::as_array)? {
        if output.get("type").and_then(Value::as_str) != Some("output") {
            continue;
        }
        let output_name = output
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        collect_output_toplevels(output, &output_name, selected_output.as_deref(), &mut out);
    }
    Some(out)
}

fn collect_output_toplevels(
    node: &Value,
    output_name: &str,
    selected_output: Option<&str>,
    out: &mut Vec<ToplevelEntry>,
) {
    if let Some(selected) = selected_output
        && !output_name.eq_ignore_ascii_case(selected)
    {
        return;
    }

    collect_toplevels_recursive(node, None, out);
}

fn collect_toplevels_recursive(
    node: &Value,
    current_workspace: Option<u32>,
    out: &mut Vec<ToplevelEntry>,
) {
    let workspace = workspace_slot_from_tree_node(node).or(current_workspace);

    if is_toplevel_node(node) {
        let title = node_title(node).unwrap_or_default().to_string();
        let app_id = node_app_id(node).unwrap_or_default().to_string();
        let identifier = node
            .get("id")
            .and_then(Value::as_i64)
            .map(|v| v.to_string())
            .unwrap_or_default();

        if !title.trim().is_empty() || !app_id.trim().is_empty() {
            let workspace_mask = workspace
                .and_then(|id| (1..=32).contains(&id).then_some(1_u32 << (id - 1)))
                .unwrap_or(0);
            out.push(ToplevelEntry {
                title,
                app_id,
                identifier,
                workspace_id: workspace,
                workspace_mask,
                focused: node
                    .get("focused")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            });
        }
    }

    for key in ["nodes", "floating_nodes"] {
        if let Some(children) = node.get(key).and_then(Value::as_array) {
            for child in children {
                collect_toplevels_recursive(child, workspace, out);
            }
        }
    }
}

fn is_toplevel_node(node: &Value) -> bool {
    let node_type = node.get("type").and_then(Value::as_str).unwrap_or_default();
    if node_type != "con" && node_type != "floating_con" {
        return false;
    }

    let has_app_or_title = node_app_id(node).is_some() || node_title(node).is_some();
    let has_window = node.get("window").and_then(Value::as_i64).unwrap_or(0) > 0;
    has_app_or_title || has_window
}

fn resolve_selected_output(selector: &str, workspaces: &Value) -> Option<String> {
    let selector = selector.trim();
    if selector.is_empty() || selector.eq_ignore_ascii_case("focused") {
        return focused_output_name(workspaces);
    }
    if let Ok(index) = selector.parse::<usize>() {
        return super::ipc::output_name_by_index(index);
    }
    Some(selector.to_string())
}

fn focused_output_name(workspaces: &Value) -> Option<String> {
    workspaces
        .as_array()?
        .iter()
        .find(|ws| ws.get("focused").and_then(Value::as_bool) == Some(true))
        .and_then(|ws| ws.get("output"))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

fn workspace_slot_from_tree_node(node: &Value) -> Option<u32> {
    if node.get("type").and_then(Value::as_str) != Some("workspace") {
        return None;
    }

    if let Some(num) = node.get("num").and_then(Value::as_i64)
        && (1..=32).contains(&num)
    {
        return Some(num as u32);
    }

    let name = node.get("name").and_then(Value::as_str)?.trim();
    let digits: String = name.chars().take_while(|c| c.is_ascii_digit()).collect();
    let parsed = digits.parse::<u32>().ok()?;
    (1..=32).contains(&parsed).then_some(parsed)
}
