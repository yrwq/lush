use std::sync::{Mutex, Once};
use std::time::Duration;

use async_channel::{Receiver, Sender};
use once_cell::sync::Lazy;

mod collect;
mod dbus;
mod menu;
mod types;
mod watcher;

pub use types::{TrayItemSnapshot, TrayMenuEntry, TrayPixmap, TraySnapshot};

pub(super) const DEFAULT_ITEM_PATH: &str = "/StatusNotifierItem";
pub(super) const WATCHER_PATH: &str = "/StatusNotifierWatcher";
pub(super) const WATCHER_SERVICES: [&str; 3] = [
    "org.kde.StatusNotifierWatcher",
    "org.freedesktop.StatusNotifierWatcher",
    "org.ayatana.StatusNotifierWatcher",
];
pub(super) const ITEM_IFACES: [&str; 3] = [
    "org.kde.StatusNotifierItem",
    "org.freedesktop.StatusNotifierItem",
    "org.ayatana.StatusNotifierItem",
];
pub(super) const ITEM_SERVICE_PREFIXES: [&str; 3] = [
    "org.kde.StatusNotifierItem-",
    "org.freedesktop.StatusNotifierItem-",
    "org.ayatana.StatusNotifierItem-",
];

static START_TRAY_COLLECTOR: Once = Once::new();
static START_TRAY_WATCHER: Once = Once::new();
static LISTENERS: Lazy<Mutex<Vec<Sender<TraySnapshot>>>> = Lazy::new(|| Mutex::new(Vec::new()));
static LATEST: Lazy<Mutex<Option<TraySnapshot>>> = Lazy::new(|| Mutex::new(None));
static REGISTERED_ITEMS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn subscribe() -> Option<Receiver<TraySnapshot>> {
    ensure_started();
    let (tx, rx) = async_channel::unbounded();
    if let Ok(latest) = LATEST.lock()
        && let Some(snapshot) = latest.clone()
    {
        let _ = tx.try_send(snapshot);
    }
    LISTENERS.lock().ok()?.push(tx);
    Some(rx)
}

pub fn activate(item: &TrayItemSnapshot, x: i32, y: i32) -> Result<(), String> {
    dbus::call_item_method(item, "Activate", &(x, y))
}

pub fn secondary_activate(item: &TrayItemSnapshot, x: i32, y: i32) -> Result<(), String> {
    dbus::call_item_method(item, "SecondaryActivate", &(x, y))
}

pub fn context_menu(item: &TrayItemSnapshot, x: i32, y: i32) -> Result<(), String> {
    dbus::call_item_method(item, "ContextMenu", &(x, y))
}

pub fn scroll(item: &TrayItemSnapshot, delta: i32) -> Result<(), String> {
    dbus::call_item_method(item, "Scroll", &(delta, "vertical"))
}

pub fn fetch_menu(item: &TrayItemSnapshot) -> Result<Vec<TrayMenuEntry>, String> {
    menu::fetch_menu(item)
}

pub fn activate_menu_item(item: &TrayItemSnapshot, id: i32) -> Result<(), String> {
    menu::activate_menu_item(item, id)
}

fn ensure_started() {
    START_TRAY_WATCHER.call_once(|| {
        if let Err(err) = std::thread::Builder::new()
            .name("lush-tray-watcher".to_string())
            .spawn(move || {
                if let Err(err) = watcher::run_watcher_server() {
                    log::warn!("tray watcher failed: {}", err);
                }
            })
        {
            log::warn!("tray watcher thread spawn failed: {}", err);
        }
    });

    START_TRAY_COLLECTOR.call_once(|| {
        if let Err(err) = std::thread::Builder::new()
            .name("lush-tray".to_string())
            .spawn(move || loop {
                match zbus::blocking::Connection::session() {
                    Ok(conn) => match collect::collect_snapshot(&conn) {
                        Ok(snapshot) => publish(snapshot),
                        Err(_err) => {}
                    },
                    Err(_err) => {}
                }
                std::thread::sleep(Duration::from_secs(2));
            })
        {
            log::warn!("tray collector thread spawn failed: {}", err);
        }
    });
}

pub(super) fn publish(snapshot: TraySnapshot) {
    if let Ok(mut latest) = LATEST.lock() {
        if latest.as_ref() == Some(&snapshot) {
            return;
        }
        *latest = Some(snapshot.clone());
    }

    if let Ok(mut listeners) = LISTENERS.lock() {
        listeners.retain(|tx| tx.try_send(snapshot.clone()).is_ok());
    }
}

pub(super) fn register_item(service_or_path: String, sender: Option<String>) {
    let value = service_or_path.trim();
    if value.is_empty() {
        return;
    }

    let normalized = if value.starts_with('/') {
        if let Some(sender) = sender.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
            format!("{sender}{value}")
        } else {
            value.to_string()
        }
    } else if let Some((service, path_tail)) = value.split_once('/') {
        if path_tail.trim().is_empty() {
            service.to_string()
        } else {
            format!("{}/{}", service.trim(), path_tail.trim_start_matches('/'))
        }
    } else {
        value.to_string()
    };

    if let Ok(mut items) = REGISTERED_ITEMS.lock()
        && !items.iter().any(|v| v == &normalized)
    {
        items.push(normalized);
    }
}

pub(super) fn registered_items_snapshot() -> Vec<String> {
    REGISTERED_ITEMS
        .lock()
        .ok()
        .map(|items| items.clone())
        .unwrap_or_default()
}
