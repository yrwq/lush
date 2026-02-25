use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;
use std::{collections::HashSet, fs, path::Path, path::PathBuf};

use crate::runtime::signal_bus::SignalBus;

use super::{DEFAULT_TIMEOUT_MS, MAX_HISTORY_SLOTS, MAX_NOTIFICATION_SLOTS, NotificationData};

static LOCAL_NOTIFICATION_ID: AtomicU32 = AtomicU32::new(10_000);
static ICON_CACHE_SEQ: AtomicU64 = AtomicU64::new(1);

pub(super) fn send(
    bus: &SignalBus,
    title: &str,
    body: &str,
    icon: Option<&str>,
    urgency: Option<&str>,
    timeout_ms: Option<i32>,
) {
    let id = LOCAL_NOTIFICATION_ID.fetch_add(1, Ordering::Relaxed);
    let urgency = parse_urgency(urgency.unwrap_or("normal"));
    let data = NotificationData {
        id,
        app_name: "lush".to_string(),
        title: title.to_string(),
        body: body.to_string(),
        icon: icon.unwrap_or("").to_string(),
        urgency,
        urgency_name: urgency_name(urgency).to_string(),
    };
    push_notification(bus, data);
    schedule_auto_hide(
        bus.clone(),
        id,
        timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS as i32),
    );
}

pub(super) fn clear_history_now(bus: &SignalBus) {
    clear_history(bus);
}

pub(super) fn delete_history_now(bus: &SignalBus, index: usize) {
    delete_history_slot(bus, index);
}

pub(super) fn push_notification(bus: &SignalBus, data: NotificationData) {
    let mut data = data;
    data.icon = persistent_icon_path(&data.icon);

    bus.batch(|| {
        for slot in (2..=MAX_NOTIFICATION_SLOTS).rev() {
            copy_slot(bus, slot - 1, slot);
        }
        write_slot(bus, 1, &data, true);
        push_history(bus, &data);
        sync_primary_from_slot(bus);
        update_counts(bus);
        emit_notification_event(bus, "pushed", Some(&data), None);
        bus.set("notification.has", "1");
        emit_notification_changed(bus);
        cleanup_cached_icons(bus);
    });
}

fn persistent_icon_path(icon: &str) -> String {
    let icon = icon.trim();
    if icon.is_empty() {
        return String::new();
    }

    let source = if let Some(raw) = icon.strip_prefix("file://") {
        raw
    } else {
        icon
    };

    let source_path = Path::new(source);
    if !source_path.is_file() {
        return icon.to_string();
    }

    let mut cache_path = cached_icon_dir();
    if fs::create_dir_all(&cache_path).is_err() {
        return icon.to_string();
    }

    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.is_empty())
        .unwrap_or("img");
    let seq = ICON_CACHE_SEQ.fetch_add(1, Ordering::Relaxed);
    cache_path.push(format!("n{}-{}.{}", seq, std::process::id(), ext));

    if fs::copy(source_path, &cache_path).is_ok() {
        return cache_path.to_string_lossy().to_string();
    }

    icon.to_string()
}

fn cached_icon_dir() -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push("lush-notification-icons");
    dir
}

fn cleanup_cached_icons(bus: &SignalBus) {
    let dir = cached_icon_dir();
    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };

    let mut referenced = HashSet::new();
    for slot in 1..=MAX_NOTIFICATION_SLOTS {
        if let Some(path) = bus.get(&slot_key(slot, "icon"))
            && is_cached_icon_path(&path, &dir)
        {
            referenced.insert(path);
        }
    }
    for slot in 1..=MAX_HISTORY_SLOTS {
        if let Some(path) = bus.get(&history_key(slot, "icon"))
            && is_cached_icon_path(&path, &dir)
        {
            referenced.insert(path);
        }
    }

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let key = path.to_string_lossy().to_string();
        if !referenced.contains(&key) {
            let _ = fs::remove_file(path);
        }
    }
}

fn is_cached_icon_path(path: &str, dir: &Path) -> bool {
    let p = Path::new(path);
    p.is_absolute() && p.starts_with(dir)
}

pub(super) fn close_notification_by_id(bus: &SignalBus, id: u32) {
    bus.batch(|| {
        let mut changed = false;
        let mut closed_data: Option<NotificationData> = None;
        for slot in 1..=MAX_NOTIFICATION_SLOTS {
            let slot_data = read_slot(bus, slot);
            let slot_id = slot_data.as_ref().map(|d| d.id).unwrap_or(0);
            if slot_id == id {
                if closed_data.is_none() {
                    closed_data = slot_data;
                }
                clear_slot(bus, slot);
                changed = true;
            }
        }

        if changed {
            compact_slots(bus);
            update_counts(bus);
            emit_notification_event(bus, "closed", closed_data.as_ref(), None);
            emit_notification_changed(bus);
            cleanup_cached_icons(bus);
        }
        sync_primary_from_slot(bus);
    });
}

pub(super) fn schedule_auto_hide(bus: SignalBus, id: u32, timeout_ms: i32) {
    let delay = if timeout_ms == -1 {
        Some(DEFAULT_TIMEOUT_MS)
    } else if timeout_ms > 0 {
        Some(timeout_ms as u64)
    } else {
        None
    };

    let Some(delay) = delay else {
        return;
    };

    glib::timeout_add_local_once(Duration::from_millis(delay), move || {
        close_notification_by_id(&bus, id);
    });
}

pub(super) fn urgency_name(value: u8) -> &'static str {
    match value {
        0 => "low",
        2 => "critical",
        _ => "normal",
    }
}

fn parse_urgency(value: &str) -> u8 {
    match value {
        "low" | "0" => 0,
        "critical" | "2" => 2,
        _ => 1,
    }
}

fn slot_key(slot: usize, field: &str) -> String {
    format!("notification.slot{}.{}", slot, field)
}

fn history_key(slot: usize, field: &str) -> String {
    format!("notification.history{}.{}", slot, field)
}

fn copy_slot(bus: &SignalBus, from: usize, to: usize) {
    for field in [
        "id",
        "app_name",
        "summary",
        "title",
        "body",
        "icon",
        "urgency",
        "urgency_name",
        "visible",
    ] {
        let value = bus.get(&slot_key(from, field)).unwrap_or_default();
        bus.set(&slot_key(to, field), &value);
    }
}

fn write_slot(bus: &SignalBus, slot: usize, data: &NotificationData, visible: bool) {
    bus.set(&slot_key(slot, "id"), &data.id.to_string());
    bus.set(&slot_key(slot, "app_name"), &data.app_name);
    bus.set(&slot_key(slot, "summary"), &data.title);
    bus.set(&slot_key(slot, "title"), &data.title);
    bus.set(&slot_key(slot, "body"), &data.body);
    bus.set(&slot_key(slot, "icon"), &data.icon);
    bus.set(&slot_key(slot, "urgency"), &data.urgency.to_string());
    bus.set(&slot_key(slot, "urgency_name"), &data.urgency_name);
    bus.set(&slot_key(slot, "visible"), if visible { "1" } else { "0" });
}

fn copy_history(bus: &SignalBus, from: usize, to: usize) {
    for field in [
        "id",
        "app_name",
        "summary",
        "title",
        "body",
        "icon",
        "urgency",
        "urgency_name",
        "visible",
    ] {
        let value = bus.get(&history_key(from, field)).unwrap_or_default();
        bus.set(&history_key(to, field), &value);
    }
}

fn write_history(bus: &SignalBus, slot: usize, data: &NotificationData, visible: bool) {
    bus.set(&history_key(slot, "id"), &data.id.to_string());
    bus.set(&history_key(slot, "app_name"), &data.app_name);
    bus.set(&history_key(slot, "summary"), &data.title);
    bus.set(&history_key(slot, "title"), &data.title);
    bus.set(&history_key(slot, "body"), &data.body);
    bus.set(&history_key(slot, "icon"), &data.icon);
    bus.set(&history_key(slot, "urgency"), &data.urgency.to_string());
    bus.set(&history_key(slot, "urgency_name"), &data.urgency_name);
    bus.set(
        &history_key(slot, "visible"),
        if visible { "1" } else { "0" },
    );
}

fn push_history(bus: &SignalBus, data: &NotificationData) {
    for slot in (2..=MAX_HISTORY_SLOTS).rev() {
        copy_history(bus, slot - 1, slot);
    }
    write_history(bus, 1, data, true);
}

fn clear_history_slot(bus: &SignalBus, slot: usize) {
    let empty = NotificationData {
        id: 0,
        app_name: String::new(),
        title: String::new(),
        body: String::new(),
        icon: String::new(),
        urgency: 1,
        urgency_name: "normal".to_string(),
    };
    write_history(bus, slot, &empty, false);
}

fn clear_history(bus: &SignalBus) {
    bus.batch(|| {
        for slot in 1..=MAX_HISTORY_SLOTS {
            clear_history_slot(bus, slot);
        }
        update_counts(bus);
        emit_notification_event(bus, "history_cleared", None, None);
        emit_notification_changed(bus);
        cleanup_cached_icons(bus);
    });
}

fn delete_history_slot(bus: &SignalBus, idx: usize) {
    if idx == 0 || idx > MAX_HISTORY_SLOTS {
        return;
    }
    bus.batch(|| {
        for slot in idx..MAX_HISTORY_SLOTS {
            copy_history(bus, slot + 1, slot);
        }
        clear_history_slot(bus, MAX_HISTORY_SLOTS);
        update_counts(bus);
        emit_notification_event(bus, "history_deleted", None, Some(idx));
        emit_notification_changed(bus);
        cleanup_cached_icons(bus);
    });
}

pub(super) fn start_history_controls(bus: SignalBus) {
    let bus_for_sub = bus.clone();
    bus_for_sub.subscribe(move |event| {
        if (event.name == "notification.history_clear"
            || event.name == "notification.history.clear")
            && is_triggered(&event.value)
        {
            clear_history(&bus);
            bus.set("notification.history_clear", "0");
            bus.set("notification.history.clear", "0");
        }

        if (event.name == "notification.history_delete"
            || event.name == "notification.history.delete")
            && event.value != "0"
            && !event.value.is_empty()
            && let Ok(idx) = event.value.parse::<usize>()
        {
            delete_history_slot(&bus, idx);
            bus.set("notification.history_delete", "0");
            bus.set("notification.history.delete", "0");
        }
        true
    });
}

fn is_triggered(value: &str) -> bool {
    !value.is_empty() && value != "0" && value != "false"
}

fn emit_notification_changed(bus: &SignalBus) {
    let next = bus
        .get("notification.changed")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0)
        .saturating_add(1);
    bus.set("notification.changed", &next.to_string());
}

fn emit_notification_event(
    bus: &SignalBus,
    kind: &str,
    data: Option<&NotificationData>,
    history_index: Option<usize>,
) {
    bus.set("notification.event", kind);
    if let Some(data) = data {
        bus.set("notification.event.id", &data.id.to_string());
        bus.set("notification.event.app_name", &data.app_name);
        bus.set("notification.event.summary", &data.title);
        bus.set("notification.event.title", &data.title);
        bus.set("notification.event.body", &data.body);
        bus.set("notification.event.icon", &data.icon);
        bus.set("notification.event.urgency", &data.urgency.to_string());
        bus.set("notification.event.urgency_name", &data.urgency_name);
    } else {
        bus.set("notification.event.id", "0");
        bus.set("notification.event.app_name", "");
        bus.set("notification.event.summary", "");
        bus.set("notification.event.title", "");
        bus.set("notification.event.body", "");
        bus.set("notification.event.icon", "");
        bus.set("notification.event.urgency", "1");
        bus.set("notification.event.urgency_name", "normal");
    }
    bus.set(
        "notification.event.history_index",
        &history_index.unwrap_or(0).to_string(),
    );
    let seq = bus
        .get("notification.event_seq")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0)
        .saturating_add(1);
    bus.set("notification.event_seq", &seq.to_string());
}

fn update_counts(bus: &SignalBus) {
    let mut active = 0usize;
    for slot in 1..=MAX_NOTIFICATION_SLOTS {
        if bus.get(&slot_key(slot, "visible")).as_deref() == Some("1") {
            active += 1;
        }
    }
    bus.set("notification.active_count", &active.to_string());

    let mut history = 0usize;
    for slot in 1..=MAX_HISTORY_SLOTS {
        if bus.get(&history_key(slot, "visible")).as_deref() == Some("1") {
            history += 1;
        }
    }
    bus.set("notification.history_count", &history.to_string());
}

fn clear_slot(bus: &SignalBus, slot: usize) {
    let empty = NotificationData {
        id: 0,
        app_name: String::new(),
        title: String::new(),
        body: String::new(),
        icon: String::new(),
        urgency: 1,
        urgency_name: "normal".to_string(),
    };
    write_slot(bus, slot, &empty, false);
}

fn read_slot(bus: &SignalBus, slot: usize) -> Option<NotificationData> {
    let id = bus
        .get(&slot_key(slot, "id"))
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    if id == 0 {
        return None;
    }

    Some(NotificationData {
        id,
        app_name: bus.get(&slot_key(slot, "app_name")).unwrap_or_default(),
        title: bus.get(&slot_key(slot, "title")).unwrap_or_default(),
        body: bus.get(&slot_key(slot, "body")).unwrap_or_default(),
        icon: bus.get(&slot_key(slot, "icon")).unwrap_or_default(),
        urgency: bus
            .get(&slot_key(slot, "urgency"))
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(1),
        urgency_name: bus
            .get(&slot_key(slot, "urgency_name"))
            .unwrap_or_else(|| "normal".to_string()),
    })
}

fn compact_slots(bus: &SignalBus) {
    let mut active = Vec::new();
    for slot in 1..=MAX_NOTIFICATION_SLOTS {
        if let Some(data) = read_slot(bus, slot) {
            let visible = bus.get(&slot_key(slot, "visible")).as_deref() == Some("1");
            if visible {
                active.push(data);
            }
        }
    }

    for slot in 1..=MAX_NOTIFICATION_SLOTS {
        clear_slot(bus, slot);
    }
    for (idx, data) in active.iter().enumerate() {
        write_slot(bus, idx + 1, data, true);
    }
}

fn sync_primary_from_slot(bus: &SignalBus) {
    let visible = bus.get(&slot_key(1, "visible")).as_deref() == Some("1");
    if !visible {
        bus.set("notification.visible", "0");
        return;
    }

    if let Some(data) = read_slot(bus, 1) {
        bus.set("notification.id", &data.id.to_string());
        bus.set("notification.app_name", &data.app_name);
        bus.set("notification.summary", &data.title);
        bus.set("notification.title", &data.title);
        bus.set("notification.body", &data.body);
        bus.set("notification.icon", &data.icon);
        bus.set("notification.urgency", &data.urgency.to_string());
        bus.set("notification.urgency_name", &data.urgency_name);
        bus.set("notification.visible", "1");
    } else {
        bus.set("notification.visible", "0");
    }
}
