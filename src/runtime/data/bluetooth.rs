use glib::SourceId;
use zbus::blocking::Connection as BlockingConnection;
use zbus::blocking::fdo::ObjectManagerProxy as BlockingObjectManagerProxy;
use zbus::export::futures_util::StreamExt;
use zbus::fdo::ManagedObjects;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MatchRule, MessageType};

use crate::runtime::signal_bus::SignalBus;

const BLUEZ_BUS_NAME: &str = "org.bluez";
const ADAPTER_IFACE: &str = "org.bluez.Adapter1";
const DEVICE_IFACE: &str = "org.bluez.Device1";
const BATTERY_IFACE: &str = "org.bluez.Battery1";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BluetoothSnapshot {
    available: bool,
    powered: bool,
    connected_count: u32,
    connected_name: String,
    connected_address: String,
    connected_battery_percent: String,
    adapter: String,
    state: String,
    summary: String,
}

pub struct BluetoothWatcher {
    task: Option<glib::JoinHandle<()>>,
    fallback: Option<SourceId>,
}

impl Drop for BluetoothWatcher {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        if let Some(source) = self.fallback.take() {
            source.remove();
        }
    }
}

pub fn start_evented(bus: SignalBus, interval_seconds: u32) -> BluetoothWatcher {
    collect(&bus);

    let fallback_interval = interval_seconds.max(30);
    let fallback_bus = bus.clone();
    let fallback = glib::timeout_add_seconds_local(fallback_interval, move || {
        collect(&fallback_bus);
        glib::ControlFlow::Continue
    });

    let task_bus = bus.clone();
    let task = glib::MainContext::default().spawn_local(async move {
        let conn = match Connection::system().await {
            Ok(conn) => conn,
            Err(err) => {
                log::debug!("data.bluetooth: failed to connect to system dbus: {}", err);
                return;
            }
        };

        let rule = match MatchRule::builder()
            .msg_type(MessageType::Signal)
            .sender(BLUEZ_BUS_NAME)
        {
            Ok(builder) => builder.path_namespace("/org/bluez").map(|b| b.build()),
            Err(err) => Err(err),
        };
        let rule = match rule {
            Ok(rule) => rule,
            Err(err) => {
                log::debug!("data.bluetooth: failed to build dbus match rule: {}", err);
                return;
            }
        };

        let mut stream = match zbus::MessageStream::for_match_rule(rule, &conn, Some(64)).await {
            Ok(stream) => stream,
            Err(err) => {
                log::debug!(
                    "data.bluetooth: failed to subscribe to bluez signals: {}",
                    err
                );
                return;
            }
        };

        while let Some(msg) = stream.next().await {
            if msg.is_ok() {
                collect(&task_bus);
            }
        }
    });

    BluetoothWatcher {
        task: Some(task),
        fallback: Some(fallback),
    }
}

fn collect(bus: &SignalBus) {
    let snapshot = match collect_snapshot() {
        Ok(snapshot) => snapshot,
        Err(err) => {
            log::debug!("data.bluetooth collector error: {}", err);
            BluetoothSnapshot {
                state: "unavailable".to_string(),
                summary: "bluetooth unavailable".to_string(),
                ..BluetoothSnapshot::default()
            }
        }
    };
    apply_snapshot(bus, &snapshot);
}

fn collect_snapshot() -> Result<BluetoothSnapshot, String> {
    let conn =
        BlockingConnection::system().map_err(|e| format!("bluetooth: dbus connect: {}", e))?;
    let proxy = BlockingObjectManagerProxy::builder(&conn)
        .destination(BLUEZ_BUS_NAME)
        .map_err(|e| format!("bluetooth: object-manager destination: {}", e))?
        .path("/")
        .map_err(|e| format!("bluetooth: object-manager path: {}", e))?
        .build()
        .map_err(|e| format!("bluetooth: object-manager proxy: {}", e))?;

    let objects: ManagedObjects = proxy
        .get_managed_objects()
        .map_err(|e| format!("bluetooth: GetManagedObjects: {}", e))?;

    let mut adapter_name = String::new();
    let mut powered = false;
    let mut has_adapter = false;

    let mut connected_count = 0u32;
    let mut connected_name = String::new();
    let mut connected_address = String::new();
    let mut connected_battery_percent = String::new();

    for interfaces in objects.values() {
        let adapter_props = interfaces.iter().find_map(|(iface_name, props)| {
            if iface_name.as_str() == ADAPTER_IFACE {
                Some(props)
            } else {
                None
            }
        });
        if let Some(adapter_props) = adapter_props {
            has_adapter = true;
            powered |= value_bool(adapter_props.get("Powered")).unwrap_or(false);
            if adapter_name.is_empty() {
                adapter_name = value_string(adapter_props.get("Alias"))
                    .or_else(|| value_string(adapter_props.get("Name")))
                    .unwrap_or_default();
            }
        }

        let device_props = interfaces.iter().find_map(|(iface_name, props)| {
            if iface_name.as_str() == DEVICE_IFACE {
                Some(props)
            } else {
                None
            }
        });
        if let Some(device_props) = device_props
            && value_bool(device_props.get("Connected")).unwrap_or(false)
        {
            connected_count = connected_count.saturating_add(1);
            if connected_name.is_empty() {
                connected_name = value_string(device_props.get("Alias"))
                    .or_else(|| value_string(device_props.get("Name")))
                    .unwrap_or_default();
                connected_address = value_string(device_props.get("Address")).unwrap_or_default();
                let battery_props = interfaces.iter().find_map(|(iface_name, props)| {
                    if iface_name.as_str() == BATTERY_IFACE {
                        Some(props)
                    } else {
                        None
                    }
                });
                if let Some(battery_props) = battery_props {
                    connected_battery_percent = value_u8(battery_props.get("Percentage"))
                        .unwrap_or(0)
                        .to_string();
                }
            }
        }
    }

    let available = has_adapter;
    let state = if !available {
        "unavailable".to_string()
    } else if !powered {
        "off".to_string()
    } else if connected_count > 0 {
        "connected".to_string()
    } else {
        "on".to_string()
    };

    let summary = if !available {
        "bluetooth unavailable".to_string()
    } else if !powered {
        "bluetooth off".to_string()
    } else if connected_count == 0 {
        "bluetooth on".to_string()
    } else if connected_name.is_empty() {
        format!("{} connected", connected_count)
    } else if connected_battery_percent.is_empty() {
        format!("{} ({})", connected_name, connected_count)
    } else {
        format!(
            "{} {}% ({})",
            connected_name, connected_battery_percent, connected_count
        )
    };

    Ok(BluetoothSnapshot {
        available,
        powered,
        connected_count,
        connected_name,
        connected_address,
        connected_battery_percent,
        adapter: adapter_name,
        state,
        summary,
    })
}

fn value_bool(value: Option<&OwnedValue>) -> Option<bool> {
    let value = value?.try_clone().ok()?;
    bool::try_from(value).ok()
}

fn value_u8(value: Option<&OwnedValue>) -> Option<u8> {
    let value = value?;
    if let Ok(v) = u8::try_from(value.try_clone().ok()?) {
        return Some(v);
    }
    if let Ok(v) = u32::try_from(value.try_clone().ok()?) {
        return Some(v.clamp(0, 100) as u8);
    }
    if let Ok(v) = i32::try_from(value.try_clone().ok()?) {
        return Some(v.clamp(0, 100) as u8);
    }
    None
}

fn value_string(value: Option<&OwnedValue>) -> Option<String> {
    let value = value?.try_clone().ok()?;
    String::try_from(value).ok().map(|v| normalize_ws(&v))
}

fn normalize_ws(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn apply_snapshot(bus: &SignalBus, snapshot: &BluetoothSnapshot) {
    bus.batch(|| {
        bus.set(
            "data.bluetooth.available",
            if snapshot.available { "1" } else { "0" },
        );
        bus.set(
            "data.bluetooth.powered",
            if snapshot.powered { "1" } else { "0" },
        );
        bus.set(
            "data.bluetooth.connected_count",
            &snapshot.connected_count.to_string(),
        );
        bus.set("data.bluetooth.connected_name", &snapshot.connected_name);
        bus.set(
            "data.bluetooth.connected_address",
            &snapshot.connected_address,
        );
        bus.set(
            "data.bluetooth.connected_battery_percent",
            &snapshot.connected_battery_percent,
        );
        bus.set("data.bluetooth.adapter", &snapshot.adapter);
        bus.set("data.bluetooth.state", &snapshot.state);
        bus.set("data.bluetooth.summary", &snapshot.summary);
    });
}
