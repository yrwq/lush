use std::collections::HashMap;

use zbus::blocking::{Connection as BlockingConnection, Proxy as BlockingProxy};
use zbus::zvariant::OwnedValue;

use super::{TrayItemSnapshot, TrayMenuEntry};

type MenuProps = HashMap<String, OwnedValue>;
type LayoutNode = (i32, MenuProps, Vec<OwnedValue>);

pub(super) fn fetch_menu(item: &TrayItemSnapshot) -> Result<Vec<TrayMenuEntry>, String> {
    let conn = BlockingConnection::session().map_err(|e| format!("dbus connect: {}", e))?;
    let Some(menu_path) = item.menu_path.as_deref() else {
        return Err("tray item has no Menu path".to_string());
    };

    let proxy = BlockingProxy::new(
        &conn,
        item.service.as_str(),
        menu_path,
        "com.canonical.dbusmenu",
    )
    .map_err(|e| format!("dbus menu proxy at {}: {}", menu_path, e))?;

    let _ = proxy.call::<_, _, bool>("AboutToShow", &(0_i32));
    let props = vec!["label", "enabled", "visible", "type"];
    let (_rev, root): (u32, LayoutNode) = proxy
        .call("GetLayout", &(0_i32, -1_i32, props))
        .map_err(|e| format!("GetLayout at {} failed: {}", menu_path, e))?;

    let mut out = Vec::new();
    for child in root.2 {
        let Ok(child) = LayoutNode::try_from(child) else {
            continue;
        };
        let id = child.0;
        if id <= 0 {
            continue;
        }

        let props = child.1;
        let separator = prop_string(&props, "type")
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case("separator"));

        out.push(TrayMenuEntry {
            id,
            label: normalize_menu_label(prop_string(&props, "label").unwrap_or_default(), id),
            enabled: prop_bool(&props, "enabled").unwrap_or(true),
            visible: prop_bool(&props, "visible").unwrap_or(true),
            separator,
        });
    }

    Ok(out)
}

pub(super) fn activate_menu_item(item: &TrayItemSnapshot, id: i32) -> Result<(), String> {
    let Some(menu_path) = item.menu_path.as_deref() else {
        return Err("tray item has no Menu path".to_string());
    };

    let conn = BlockingConnection::session().map_err(|e| format!("dbus connect: {}", e))?;
    let proxy = BlockingProxy::new(
        &conn,
        item.service.as_str(),
        menu_path,
        "com.canonical.dbusmenu",
    )
    .map_err(|e| format!("dbus menu proxy: {}", e))?;

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let timestamp = (now_ms % u32::MAX as u64) as u32;

    proxy
        .call::<_, _, ()>("Event", &(id, "clicked", OwnedValue::from(0_i32), timestamp))
        .map_err(|e| format!("menu Event failed: {}", e))?;
    Ok(())
}

fn prop_string(props: &MenuProps, key: &str) -> Option<String> {
    let raw = props.get(key)?;

    if let Ok(value) = <&str>::try_from(raw) {
        let s = value.trim();
        if !s.is_empty() {
            return Some(s.to_string());
        }
    }
    if let Ok(cloned) = raw.try_clone()
        && let Ok(value) = String::try_from(cloned)
    {
        let s = value.trim();
        if !s.is_empty() {
            return Some(s.to_string());
        }
    }
    None
}

fn prop_bool(props: &MenuProps, key: &str) -> Option<bool> {
    let raw = props.get(key)?;

    if let Ok(v) = bool::try_from(raw) {
        return Some(v);
    }
    if let Ok(v) = i32::try_from(raw.try_clone().ok()?) {
        return Some(v != 0);
    }
    if let Ok(v) = u32::try_from(raw.try_clone().ok()?) {
        return Some(v != 0);
    }
    if let Ok(v) = String::try_from(raw.try_clone().ok()?) {
        let n = v.trim().to_ascii_lowercase();
        return Some(!matches!(n.as_str(), "" | "0" | "false" | "off" | "no"));
    }

    None
}

fn normalize_menu_label(raw: String, id: i32) -> String {
    let text = raw.replace('_', "").trim().to_string();
    if text.is_empty() {
        format!("item {}", id)
    } else {
        text
    }
}
