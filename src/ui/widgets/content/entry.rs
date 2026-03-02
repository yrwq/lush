use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gtk4::prelude::*;
use gtk4::{Entry, Widget};

use crate::config::{WidgetConfig, WidgetProps};
use crate::runtime::signal_bus::SignalBus;
use crate::ui::signal_watch::watch_signal;
use crate::ui::widgets::core::bindings::initial_value_from_bind_or;
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::finalize_widget;

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Entry(props) = &cfg.props else {
        unreachable!("entry builder received non-entry props");
    };

    let fallback = props.text.clone().unwrap_or_default();
    let initial_value = initial_value_from_bind_or(props.bind.as_deref(), ctx.bus, fallback);
    let write_bind = props.input_bind.clone().or_else(|| props.bind.clone());

    let entry = Entry::new();
    entry.set_text(&initial_value);
    if let Some(placeholder) = props.placeholder.as_deref() {
        entry.set_placeholder_text(Some(placeholder));
    }
    if let Some(max_chars) = props.max_chars {
        entry.set_max_length(max_chars.max(0));
    }

    let syncing_from_bus = Rc::new(Cell::new(false));

    if let Some(bind_name) = props.bind.clone() {
        let entry = entry.clone();
        let syncing_for_watch = syncing_from_bus.clone();
        watch_signal(ctx.bus, bind_name, move |value| {
            if entry.text().as_str() == value {
                return glib::ControlFlow::Continue;
            }
            syncing_for_watch.set(true);
            entry.set_text(value);
            syncing_for_watch.set(false);
            glib::ControlFlow::Continue
        });
    }

    if let Some(target_bind) = write_bind.clone() {
        let bus_for_change = ctx.bus.clone();
        let syncing_for_change = syncing_from_bus.clone();
        entry.connect_changed(move |entry| {
            if syncing_for_change.get() {
                return;
            }
            bus_for_change.set(&target_bind, entry.text().as_str());
        });
    }

    if let Some(activate_bind) = props.activate_bind.clone() {
        let bus_for_activate = ctx.bus.clone();
        entry.connect_activate(move |entry| {
            publish_entry_activate(&bus_for_activate, &activate_bind, entry.text().as_str());
        });
    }

    if props.autofocus.unwrap_or(false) {
        entry.connect_map(|entry| {
            entry.grab_focus();
            let weak = entry.downgrade();
            for delay_ms in [16_u64, 60, 140] {
                let weak = weak.clone();
                glib::timeout_add_local_once(Duration::from_millis(delay_ms), move || {
                    if let Some(entry) = weak.upgrade() {
                        entry.grab_focus();
                    }
                });
            }
        });
    }

    let widget: Widget = entry.upcast();
    finalize_widget(&widget, cfg, ctx.bus, false);
    widget
}

fn publish_entry_activate(bus: &SignalBus, bind_name: &str, value: &str) {
    bus.set(bind_name, value);
    let seq_key = format!("{}.__user_seq", bind_name);
    let next_seq = bus
        .get(&seq_key)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0)
        .saturating_add(1);
    bus.set(&seq_key, &next_seq.to_string());
}
