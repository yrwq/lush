use std::collections::{HashMap, HashSet};

use zbus::blocking::fdo::DBusProxy as BlockingDBusProxy;
use zbus::blocking::{Connection as BlockingConnection, Proxy as BlockingProxy};
use zbus::zvariant::OwnedValue;

use super::dbus::extract_object_path_string;
use super::{
    DEFAULT_ITEM_PATH, ITEM_IFACES, ITEM_SERVICE_PREFIXES, TrayItemSnapshot, TrayPixmap, TraySnapshot,
    WATCHER_SERVICES,
};

type PropsMap = HashMap<String, OwnedValue>;

pub(super) fn collect_snapshot(conn: &BlockingConnection) -> Result<TraySnapshot, String> {
    let mut seen = HashSet::<(String, String)>::new();
    let mut descriptors = Vec::<(String, String)>::new();
    let all_names = collect_all_names(conn)?;

    for (service, path) in collect_registered_descriptors(conn) {
        if service == "*" {
            for name in &all_names {
                descriptors.push((name.clone(), path.clone()));
            }
        } else {
            descriptors.push((service, path));
        }
    }

    for service in collect_prefixed_services(conn)? {
        descriptors.push((service, DEFAULT_ITEM_PATH.to_string()));
    }

    let mut items = Vec::new();
    for (service, path) in descriptors {
        if !seen.insert((service.clone(), path.clone())) {
            continue;
        }
        if let Some(item) = collect_item(conn, &service, &path) {
            items.push(item);
        }
    }

    items.sort_by(|a, b| {
        a.status
            .cmp(&b.status)
            .then_with(|| a.id.cmp(&b.id))
            .then_with(|| a.service.cmp(&b.service))
    });

    Ok(TraySnapshot { items })
}

fn collect_registered_descriptors(conn: &BlockingConnection) -> Vec<(String, String)> {
    let mut out = Vec::new();

    for watcher in WATCHER_SERVICES {
        let Ok(proxy) = BlockingProxy::new(conn, watcher, super::WATCHER_PATH, watcher) else {
            continue;
        };

        let Ok(registered) = proxy.get_property::<Vec<String>>("RegisteredStatusNotifierItems") else {
            continue;
        };

        for raw in registered {
            if let Some((service, path)) = parse_descriptor(&raw) {
                out.push((service, path));
            }
        }
    }

    out
}

fn collect_prefixed_services(conn: &BlockingConnection) -> Result<Vec<String>, String> {
    Ok(collect_all_names(conn)?
        .into_iter()
        .filter(|name| {
            ITEM_SERVICE_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
        .collect())
}

fn collect_all_names(conn: &BlockingConnection) -> Result<Vec<String>, String> {
    let dbus = BlockingDBusProxy::new(conn).map_err(|e| format!("dbus proxy: {}", e))?;
    let names = dbus
        .list_names()
        .map_err(|e| format!("ListNames failed: {}", e))?;
    Ok(names.into_iter().map(|name| name.to_string()).collect())
}

fn parse_descriptor(raw: &str) -> Option<(String, String)> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }

    if value.starts_with('/') {
        return Some(("*".to_string(), value.to_string()));
    }

    if let Some((service, path_tail)) = value.split_once('/')
        && !service.is_empty()
    {
        let path = format!("/{}", path_tail.trim_start_matches('/'));
        return Some((service.to_string(), path));
    }

    Some((value.to_string(), DEFAULT_ITEM_PATH.to_string()))
}

fn collect_item(conn: &BlockingConnection, service: &str, path: &str) -> Option<TrayItemSnapshot> {
    let properties = BlockingProxy::new(conn, service, path, "org.freedesktop.DBus.Properties").ok()?;

    let mut selected_iface = None;
    let mut all_props = None;
    for iface in ITEM_IFACES {
        let Ok(props) = properties.call::<_, _, PropsMap>("GetAll", &(iface)) else {
            continue;
        };
        if !props.is_empty() {
            selected_iface = Some(iface);
            all_props = Some(props);
            break;
        }
    }

    let iface = selected_iface?;
    let props = all_props?;
    let menu_path = get_menu_path_any_iface(&properties).or_else(|| map_object_path(&props, "Menu"));

    Some(TrayItemSnapshot {
        service: service.to_string(),
        path: path.to_string(),
        iface: iface.to_string(),
        id: map_string(&props, "Id").unwrap_or_else(|| service.to_string()),
        title: map_string(&props, "Title").unwrap_or_default(),
        status: map_string(&props, "Status")
            .map(|v| v.to_ascii_lowercase())
            .unwrap_or_else(|| "active".to_string()),
        icon_name: map_string(&props, "IconName").unwrap_or_default(),
        attention_icon_name: map_string(&props, "AttentionIconName").unwrap_or_default(),
        icon_pixmap: map_pixmap(&props, "IconPixmap"),
        attention_icon_pixmap: map_pixmap(&props, "AttentionIconPixmap"),
        menu_path,
    })
}

fn get_menu_path(properties: &BlockingProxy<'_>, iface: &str) -> Option<String> {
    let value: OwnedValue = properties.call("Get", &(iface, "Menu")).ok()?;
    extract_object_path_string(&value)
}

fn get_menu_path_any_iface(properties: &BlockingProxy<'_>) -> Option<String> {
    for iface in ITEM_IFACES {
        if let Some(path) = get_menu_path(properties, iface) {
            let p = path.trim();
            if !p.is_empty() && p != "/" {
                return Some(path);
            }
        }
    }
    None
}

fn map_string(props: &PropsMap, key: &str) -> Option<String> {
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

fn map_object_path(props: &PropsMap, key: &str) -> Option<String> {
    let value = props.get(key)?;
    extract_object_path_string(value)
}

fn map_pixmap(props: &PropsMap, key: &str) -> Option<TrayPixmap> {
    let pixmaps = props
        .get(key)
        .and_then(|v| v.try_clone().ok())
        .and_then(|v| Vec::<(i32, i32, Vec<u8>)>::try_from(v).ok())?;

    pick_best_pixmap(pixmaps)
}

fn pick_best_pixmap(candidates: Vec<(i32, i32, Vec<u8>)>) -> Option<TrayPixmap> {
    let mut out = None::<TrayPixmap>;
    let mut best_score = i64::MIN;

    for (w, h, argb) in candidates {
        if w <= 0 || h <= 0 {
            continue;
        }
        let needed = (w as usize).saturating_mul(h as usize).saturating_mul(4);
        if argb.len() < needed {
            continue;
        }

        let area = (w as i64) * (h as i64);
        if area <= best_score {
            continue;
        }

        best_score = area;
        out = Some(TrayPixmap {
            width: w,
            height: h,
            argb,
        });
    }

    out
}
