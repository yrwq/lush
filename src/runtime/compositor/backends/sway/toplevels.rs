use serde_json::Value;

use super::ipc::{SWAY_IPC_GET_TREE, SWAY_IPC_GET_WORKSPACES, request_json};
use super::tree::{node_app_id, node_title};
use crate::runtime::compositor::ToplevelEntry;

pub(super) fn collect_toplevels(output_selector: Option<&str>) -> Option<Vec<ToplevelEntry>> {
    let tree = request_json(SWAY_IPC_GET_TREE, "")?;
    let workspaces = request_json(SWAY_IPC_GET_WORKSPACES, "")?;

    let selected_output = match output_selector {
        Some(selector) => super::resolve_selected_output(selector.trim(), &workspaces),
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
