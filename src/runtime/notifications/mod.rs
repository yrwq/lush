mod build;
mod control;

use super::signal_bus::SignalBus;

pub(super) const BUS_NAME: &str = "org.freedesktop.Notifications";
pub(super) const BUS_PATH: &str = "/org/freedesktop/Notifications";
pub(super) const DEFAULT_TIMEOUT_MS: u64 = 5000;
pub(super) const MAX_NOTIFICATION_SLOTS: usize = 3;
pub(super) const MAX_HISTORY_SLOTS: usize = 32;

#[derive(Clone, Debug)]
pub(super) struct NotificationData {
    pub(super) id: u32,
    pub(super) app_name: String,
    pub(super) title: String,
    pub(super) body: String,
    pub(super) icon: String,
    pub(super) urgency: u8,
    pub(super) urgency_name: String,
}

pub fn start(bus: SignalBus) {
    build::start(bus);
}

pub fn send(
    bus: &SignalBus,
    title: &str,
    body: &str,
    icon: Option<&str>,
    urgency: Option<&str>,
    timeout_ms: Option<i32>,
) {
    control::send(bus, title, body, icon, urgency, timeout_ms);
}

pub fn clear_history_now(bus: &SignalBus) {
    control::clear_history_now(bus);
}

pub fn delete_history_now(bus: &SignalBus, index: usize) {
    control::delete_history_now(bus, index);
}
