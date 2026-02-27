use std::cell::RefCell;
use std::rc::Rc;

use gtk4::Widget;
use gtk4::prelude::*;

use crate::config::{WidgetBase, WidgetConfig};
use crate::runtime::signal_bus::SignalBus;
use crate::ui::signal_watch::{watch_signal, window_is_visible};

pub fn finalize_widget(widget: &Widget, cfg: &WidgetConfig, bus: &SignalBus, clickable: bool) {
    apply_clickable_cursor(widget, clickable);
    apply_common(widget, cfg);
    apply_visibility(widget, &cfg.base, bus);
    apply_state_class(widget, &cfg.base, bus);
}

pub fn apply_common(widget: &Widget, cfg: &WidgetConfig) {
    widget.add_css_class("widget");
    widget.add_css_class(cfg.kind.css_class());

    for class in &cfg.base.classes {
        widget.add_css_class(class);
    }

    if let Some(v) = cfg.base.hexpand {
        widget.set_hexpand(v);
    }

    if let Some(v) = cfg.base.vexpand {
        widget.set_vexpand(v);
    }

    if let Some(v) = cfg.base.width {
        widget.set_width_request(v);
    }

    if let Some(v) = cfg.base.height {
        widget.set_height_request(v);
    }

    if let Some(ref h) = cfg.base.halign {
        widget.set_halign(parse_align(h));
    }

    if let Some(ref v) = cfg.base.valign {
        widget.set_valign(parse_align(v));
    }
}

fn parse_align(value: &str) -> gtk4::Align {
    match value {
        "start" => gtk4::Align::Start,
        "center" => gtk4::Align::Center,
        "end" => gtk4::Align::End,
        _ => gtk4::Align::Fill,
    }
}

pub fn format_text(text: &str, angle: Option<f64>) -> String {
    if let Some(a) = angle
        && ((a - 90.0).abs() < 0.1 || (a + 90.0).abs() < 0.1)
    {
        return text
            .chars()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("\n");
    }
    text.to_string()
}

fn apply_visibility(widget: &Widget, base: &WidgetBase, bus: &SignalBus) {
    if let Some(v) = base.visible {
        widget.set_visible(v);
    }

    let Some(signal_name) = base.visible_bind.clone() else {
        return;
    };
    if let Some(value) = bus.get(&signal_name) {
        widget.set_visible(parse_visible_value(&value));
    }

    let weak = widget.downgrade();
    watch_signal(bus, signal_name, move |value| {
        if let Some(widget) = weak.upgrade() {
            if !window_is_visible(&widget) {
                return glib::ControlFlow::Continue;
            }
            widget.set_visible(parse_visible_value(value));
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });
}

pub fn apply_clickable_cursor(widget: &Widget, clickable: bool) {
    if clickable {
        widget.set_cursor_from_name(Some("pointer"));
    } else {
        widget.set_cursor_from_name(None);
    }
}

fn apply_state_class(widget: &Widget, base: &WidgetBase, bus: &SignalBus) {
    let Some(signal_name) = base.class_bind.clone() else {
        return;
    };
    let prefix = "state-";
    let previous: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    if let Some(value) = bus.get(&signal_name) {
        set_state_class(widget, prefix, &value, &previous);
    }

    let weak = widget.downgrade();
    watch_signal(bus, signal_name, move |value| {
        if let Some(widget) = weak.upgrade() {
            if !window_is_visible(&widget) {
                return glib::ControlFlow::Continue;
            }
            set_state_class(&widget, prefix, value, &previous);
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });
}

fn set_state_class(
    widget: &Widget,
    prefix: &str,
    raw_value: &str,
    previous: &Rc<RefCell<Option<String>>>,
) {
    if let Some(old) = previous.borrow_mut().take() {
        widget.remove_css_class(&old);
    }

    let suffix = sanitize_css_suffix(raw_value);
    if suffix.is_empty() {
        return;
    }

    let class_name = format!("{}{}", prefix, suffix);
    widget.add_css_class(&class_name);
    *previous.borrow_mut() = Some(class_name);
}

fn sanitize_css_suffix(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn parse_visible_value(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" | "0" | "false" | "off" | "no" | "hidden" => false,
        "1" | "true" | "on" | "yes" | "visible" | "show" => true,
        _ => true,
    }
}
