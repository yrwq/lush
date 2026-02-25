use std::cell::RefCell;
use std::rc::Rc;

use gtk4::Widget;
use gtk4::prelude::*;

use crate::config::WidgetConfig;
use crate::runtime::signal_bus::SignalBus;
use crate::ui::signal_watch::watch_signal;

pub fn initial_value_from_bind_or(
    bind_name: Option<&str>,
    bus: &SignalBus,
    fallback: String,
) -> String {
    bind_name.and_then(|name| bus.get(name)).unwrap_or(fallback)
}

pub fn initial_state_from_class_bind(class_bind: Option<&str>, bus: &SignalBus) -> String {
    class_bind
        .and_then(|name| bus.get(name))
        .unwrap_or_else(|| "default".to_string())
}

pub fn watch_bind_value<W, F>(
    widget: &W,
    cfg: &WidgetConfig,
    bind_name: Option<String>,
    bus: &SignalBus,
    current_value: Rc<RefCell<String>>,
    current_state: Rc<RefCell<String>>,
    apply: F,
) where
    W: IsA<Widget> + Clone + 'static,
    F: Fn(&W, &WidgetConfig, &str, &str) + Clone + 'static,
{
    let Some(bind_name) = bind_name else {
        return;
    };

    let weak = widget.downgrade();
    let cfg_for_watch = cfg.clone();
    let value_for_watch = current_value.clone();
    let state_for_watch = current_state.clone();
    let apply_for_watch = apply.clone();
    watch_signal(bus, bind_name, move |value| {
        *value_for_watch.borrow_mut() = value.to_string();
        if let Some(widget) = weak.upgrade() {
            apply_for_watch(
                &widget,
                &cfg_for_watch,
                &value_for_watch.borrow(),
                &state_for_watch.borrow(),
            );
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });
}

pub fn watch_class_state<W, F>(
    widget: &W,
    cfg: &WidgetConfig,
    class_bind: Option<String>,
    bus: &SignalBus,
    current_value: Rc<RefCell<String>>,
    current_state: Rc<RefCell<String>>,
    apply: F,
) where
    W: IsA<Widget> + Clone + 'static,
    F: Fn(&W, &WidgetConfig, &str, &str) + Clone + 'static,
{
    let Some(state_name) = class_bind else {
        return;
    };

    let weak = widget.downgrade();
    let cfg_for_watch = cfg.clone();
    let value_for_watch = current_value.clone();
    let state_for_watch = current_state.clone();
    let apply_for_watch = apply.clone();
    watch_signal(bus, state_name, move |state| {
        *state_for_watch.borrow_mut() = state.to_string();
        if let Some(widget) = weak.upgrade() {
            apply_for_watch(
                &widget,
                &cfg_for_watch,
                &value_for_watch.borrow(),
                &state_for_watch.borrow(),
            );
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });
}
