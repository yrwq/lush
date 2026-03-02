use crate::runtime::signal_bus::SignalBus;

pub fn watch_signal<F>(bus: &SignalBus, signal_name: String, mut on_signal: F) -> u64
where
    F: FnMut(&str) -> glib::ControlFlow + 'static,
{
    bus.subscribe_key(&signal_name, move |event| {
        if matches!(on_signal(&event.value), glib::ControlFlow::Break) {
            return false;
        }
        true
    })
}
