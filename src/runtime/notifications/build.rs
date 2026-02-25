use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use glib::MainContext;
use zbus::blocking::connection::Builder as ConnectionBuilder;
use zbus::interface;
use zbus::zvariant::OwnedValue;

use crate::runtime::signal_bus::SignalBus;

use super::control;
use super::{BUS_NAME, BUS_PATH, NotificationData};

static STARTED: AtomicBool = AtomicBool::new(false);
thread_local! {
    static ACTIVE_BUS: RefCell<Option<SignalBus>> = const { RefCell::new(None) };
}

enum DbusCommand {
    Notify {
        data: NotificationData,
        expire_timeout: i32,
        reply: mpsc::Sender<u32>,
    },
    Close {
        id: u32,
    },
}

struct NotificationsIface {
    tx: async_channel::Sender<DbusCommand>,
    next_id: AtomicU32,
}

impl NotificationsIface {
    fn new(tx: async_channel::Sender<DbusCommand>) -> Self {
        Self {
            tx,
            next_id: AtomicU32::new(1),
        }
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationsIface {
    #[zbus(name = "GetCapabilities")]
    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "body".to_string(),
            "body-markup".to_string(),
            "icon-static".to_string(),
        ]
    }

    #[zbus(name = "GetServerInformation")]
    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "lush".to_string(),
            "lush".to_string(),
            "0.1.0".to_string(),
            "1.2".to_string(),
        )
    }

    #[zbus(name = "CloseNotification")]
    fn close_notification(&self, id: u32) -> zbus::fdo::Result<()> {
        self.tx
            .send_blocking(DbusCommand::Close { id })
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    #[zbus(name = "Notify")]
    fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        _actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> zbus::fdo::Result<u32> {
        let id = if replaces_id == 0 {
            self.next_id.fetch_add(1, Ordering::Relaxed)
        } else {
            replaces_id
        };
        let urgency = extract_urgency(&hints).unwrap_or(1);
        let data = NotificationData {
            id,
            app_name,
            title: summary,
            body,
            icon: extract_image_path(&app_icon, &hints),
            urgency,
            urgency_name: control::urgency_name(urgency).to_string(),
        };

        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send_blocking(DbusCommand::Notify {
                data,
                expire_timeout,
                reply: reply_tx,
            })
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        reply_rx
            .recv()
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }
}

pub(super) fn start(bus: SignalBus) {
    set_active_bus(bus.clone());
    control::start_history_controls(bus);

    if STARTED.swap(true, Ordering::AcqRel) {
        return;
    }

    let (tx, rx) = async_channel::unbounded::<DbusCommand>();
    MainContext::default().spawn_local(async move {
        while let Ok(cmd) = rx.recv().await {
            match cmd {
                DbusCommand::Notify {
                    data,
                    expire_timeout,
                    reply,
                } => {
                    let id = data.id;
                    if with_active_bus(|active_bus| {
                        control::push_notification(active_bus, data.clone());
                        control::schedule_auto_hide(active_bus.clone(), id, expire_timeout);
                    })
                    .is_none()
                    {
                        log::warn!("notifications: active signal bus unavailable; dropping notify");
                    }
                    let _ = reply.send(id);
                }
                DbusCommand::Close { id } => {
                    if with_active_bus(|active_bus| {
                        control::close_notification_by_id(active_bus, id);
                    })
                    .is_none()
                    {
                        log::warn!("notifications: active signal bus unavailable; dropping close");
                    }
                }
            }
        }
    });

    std::thread::spawn(move || {
        if let Err(err) = run_server(tx) {
            log::warn!("notifications: failed to start zbus service: {}", err);
        }
    });
}

fn set_active_bus(bus: SignalBus) {
    ACTIVE_BUS.with(|slot| {
        *slot.borrow_mut() = Some(bus);
    });
}

fn with_active_bus<R>(f: impl FnOnce(&SignalBus) -> R) -> Option<R> {
    ACTIVE_BUS.with(|slot| slot.borrow().as_ref().map(f))
}

fn run_server(tx: async_channel::Sender<DbusCommand>) -> zbus::Result<()> {
    let _connection = ConnectionBuilder::session()?
        .name(BUS_NAME)?
        .serve_at(BUS_PATH, NotificationsIface::new(tx))?
        .build()?;

    log::info!("notifications: serving {} via zbus", BUS_NAME);

    loop {
        std::thread::park_timeout(Duration::from_secs(3600));
    }
}

fn extract_image_path(app_icon: &str, hints: &HashMap<String, OwnedValue>) -> String {
    hint_string(hints, "image-path")
        .or_else(|| hint_string(hints, "image_path"))
        .unwrap_or_else(|| app_icon.to_string())
}

fn extract_urgency(hints: &HashMap<String, OwnedValue>) -> Option<u8> {
    let value = hints.get("urgency")?;
    if let Ok(v) = u8::try_from(value) {
        return Some(v);
    }
    if let Ok(v) = u32::try_from(value) {
        return Some(v as u8);
    }
    if let Ok(v) = i32::try_from(value) {
        return Some(v as u8);
    }
    None
}

fn hint_string(hints: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let value = hints.get(key)?.try_clone().ok()?;
    String::try_from(value).ok()
}
