use serde_json::Value;

pub(super) fn find_focused_node(node: &Value) -> Option<&Value> {
    for key in ["nodes", "floating_nodes"] {
        if let Some(children) = node.get(key).and_then(Value::as_array) {
            for child in children {
                if let Some(found) = find_focused_node(child) {
                    return Some(found);
                }
            }
        }
    }

    if is_focused_window_node(node) {
        return Some(node);
    }

    None
}

fn is_focused_window_node(node: &Value) -> bool {
    if node.get("focused").and_then(Value::as_bool) != Some(true) {
        return false;
    }

    let node_type = node.get("type").and_then(Value::as_str).unwrap_or_default();
    if node_type != "con" && node_type != "floating_con" {
        return false;
    }

    let has_window = node.get("window").and_then(Value::as_i64).unwrap_or(0) > 0;
    has_window || node_app_id(node).is_some() || node_title(node).is_some()
}

pub(super) fn node_app_id(node: &Value) -> Option<&str> {
    node.get("app_id")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            node.get("window_properties")
                .and_then(|v| v.get("class"))
                .and_then(Value::as_str)
                .filter(|v| !v.trim().is_empty())
        })
}

pub(super) fn node_title(node: &Value) -> Option<&str> {
    node.get("name")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            node.get("window_properties")
                .and_then(|v| v.get("title"))
                .and_then(Value::as_str)
                .filter(|v| !v.trim().is_empty())
        })
        .or_else(|| node_app_id(node))
}
