use async_channel::Receiver;
use serde_json::Value;
use std::thread;

use super::ipc::{
    SWAY_IPC_GET_TREE, SWAY_IPC_GET_WORKSPACES, read_message, request_json, subscribe_events,
};
use super::tree::{find_focused_node, node_app_id, node_title};
use crate::runtime::compositor::{CompositorSnapshot, FocusedWindowSnapshot};

pub(super) fn subscribe_snapshots(
    output_selector: Option<&str>,
) -> Option<Receiver<CompositorSnapshot>> {
    if !super::available() {
        return None;
    }

    let selector = output_selector.map(str::to_string);
    let (tx, rx) = async_channel::unbounded();
    thread::Builder::new()
        .name("sway-workspaces-listener".to_string())
        .spawn(move || {
            let mut event_stream = match subscribe_events(["workspace", "window"]) {
                Some(v) => v,
                None => return,
            };
            let mut last: Option<CompositorSnapshot> = None;

            if let Some(snapshot) = collect_snapshot(selector.as_deref()) {
                last = Some(snapshot.clone());
                if tx.send_blocking(snapshot).is_err() {
                    return;
                }
            }

            loop {
                if read_message(&mut event_stream).is_err() {
                    break;
                }

                let Some(snapshot) = collect_snapshot(selector.as_deref()) else {
                    continue;
                };

                if last.as_ref() != Some(&snapshot) {
                    last = Some(snapshot.clone());
                    if tx.send_blocking(snapshot).is_err() {
                        break;
                    }
                }
            }
        })
        .ok()?;

    Some(rx)
}

fn collect_snapshot(output_selector: Option<&str>) -> Option<CompositorSnapshot> {
    let workspaces = request_json(SWAY_IPC_GET_WORKSPACES, "")?;
    let tree = request_json(SWAY_IPC_GET_TREE, "")?;

    let mut focused_mask = 0_u32;
    let mut occupied_mask = 0_u32;
    let mut urgent_mask = 0_u32;

    let selector = output_selector.map(str::trim).unwrap_or("");
    let selected_output = resolve_selected_output(selector, &workspaces);

    let mut unnamed_slot = 1_u32;
    for ws in workspaces.as_array()? {
        let Some(ws_num) = workspace_slot(ws, &mut unnamed_slot) else {
            continue;
        };
        if !(1..=32).contains(&ws_num) {
            continue;
        }

        if let Some(selected_output) = selected_output.as_deref() {
            let out_name = ws.get("output").and_then(Value::as_str).unwrap_or("");
            if !out_name.eq_ignore_ascii_case(selected_output) {
                continue;
            }
        }

        let bit = 1_u32 << (ws_num - 1);
        occupied_mask |= bit;
        if ws.get("focused").and_then(Value::as_bool) == Some(true) {
            focused_mask |= bit;
        }
        if ws.get("urgent").and_then(Value::as_bool) == Some(true) {
            urgent_mask |= bit;
        }
    }

    let focused = find_focused_node(&tree);
    let focused_title = focused.and_then(node_title).unwrap_or_default().to_string();
    let focused_app_id = focused
        .and_then(node_app_id)
        .unwrap_or_default()
        .to_string();
    let focused_identifier = focused
        .and_then(|node| node.get("id"))
        .and_then(Value::as_i64)
        .map(|v| v.to_string())
        .unwrap_or_default();

    Some(CompositorSnapshot {
        focused_mask,
        occupied_mask,
        urgent_mask,
        focused_window: FocusedWindowSnapshot {
            title: focused_title,
            app_id: focused_app_id,
            identifier: focused_identifier,
        },
    })
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

fn workspace_slot(ws: &Value, unnamed_slot: &mut u32) -> Option<u32> {
    if let Some(num) = ws.get("num").and_then(Value::as_i64)
        && (1..=32).contains(&num)
    {
        return Some(num as u32);
    }

    let name = ws.get("name").and_then(Value::as_str)?.trim();
    let digits: String = name.chars().take_while(|c| c.is_ascii_digit()).collect();
    let parsed = digits.parse::<u32>().ok()?;
    if (1..=32).contains(&parsed) {
        return Some(parsed);
    }

    if *unnamed_slot > 32 {
        return None;
    }
    let out = *unnamed_slot;
    *unnamed_slot += 1;
    Some(out)
}
