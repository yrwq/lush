use crate::runtime::signal_bus::SignalBus;
use gtk4::Widget;
use gtk4::prelude::*;

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

pub fn window_is_visible(widget: &impl IsA<Widget>) -> bool {
    let Some(root) = widget.root() else {
        return true;
    };
    let Ok(window) = root.downcast::<gtk4::Window>() else {
        return true;
    };
    window.is_visible()
}
