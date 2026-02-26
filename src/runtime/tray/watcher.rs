use std::time::Duration;

use zbus::blocking::connection::Builder as ConnectionBuilder;
use zbus::interface;
use zbus::message::Header;

use super::{register_item, registered_items_snapshot, WATCHER_PATH, WATCHER_SERVICES};

macro_rules! define_watcher_iface {
    ($struct_name:ident, $iface_name:literal) => {
        struct $struct_name;

        #[interface(name = $iface_name)]
        impl $struct_name {
            #[zbus(name = "RegisterStatusNotifierItem")]
            fn register_status_notifier_item(
                &self,
                service_or_path: String,
                #[zbus(header)] header: Header<'_>,
            ) -> zbus::fdo::Result<()> {
                register_item(service_or_path, header.sender().map(|v| v.to_string()));
                Ok(())
            }

            #[zbus(name = "RegisterStatusNotifierHost")]
            fn register_status_notifier_host(&self, _service: String) -> zbus::fdo::Result<()> {
                Ok(())
            }

            #[zbus(property, name = "RegisteredStatusNotifierItems")]
            fn registered_status_notifier_items(&self) -> Vec<String> {
                registered_items_snapshot()
            }

            #[zbus(property, name = "IsStatusNotifierHostRegistered")]
            fn is_status_notifier_host_registered(&self) -> bool {
                true
            }

            #[zbus(property, name = "ProtocolVersion")]
            fn protocol_version(&self) -> i32 {
                0
            }
        }
    };
}

define_watcher_iface!(KdeStatusNotifierWatcherIface, "org.kde.StatusNotifierWatcher");
define_watcher_iface!(
    FreedesktopStatusNotifierWatcherIface,
    "org.freedesktop.StatusNotifierWatcher"
);
define_watcher_iface!(AyatanaStatusNotifierWatcherIface, "org.ayatana.StatusNotifierWatcher");

pub(super) fn run_watcher_server() -> zbus::Result<()> {
    let connection = ConnectionBuilder::session()?
        .serve_at(WATCHER_PATH, KdeStatusNotifierWatcherIface)?
        .serve_at(WATCHER_PATH, FreedesktopStatusNotifierWatcherIface)?
        .serve_at(WATCHER_PATH, AyatanaStatusNotifierWatcherIface)?
        .build()?;

    let mut acquired = 0_u8;
    for name in WATCHER_SERVICES {
        match connection.request_name(name) {
            Ok(_) => {
                acquired += 1;
                log::info!("tray watcher: acquired bus name {}", name);
            }
            Err(_err) => {}
        }
    }

    if acquired == 0 {
        log::warn!("tray watcher: no well-known watcher bus name acquired");
    }

    loop {
        std::thread::park_timeout(Duration::from_secs(3600));
    }
}
