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

    if node.get("focused").and_then(Value::as_bool) == Some(true) {
        return Some(node);
    }

    None
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
