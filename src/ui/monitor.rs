use gio::prelude::ListModelExt;
use gtk4::ApplicationWindow;
use gtk4::prelude::*;
use gtk4_layer_shell::LayerShell;

pub(crate) fn resolve_widget_output_selector(output: Option<&str>) -> Option<String> {
    let selector = output?;
    let Ok(_) = selector.parse::<u32>() else {
        return Some(selector.to_string());
    };

    let display = gtk4::gdk::Display::default()?;
    find_monitor_by_selector(&display, selector)?
        .connector()
        .map(|v| v.to_string())
        .or_else(|| Some(selector.to_string()))
}

pub(crate) fn apply_output_target(window: &ApplicationWindow, output: Option<&str>) {
    let Some(selector) = output else {
        return;
    };

    let Some(display) = gtk4::gdk::Display::default() else {
        log::warn!(
            "window output '{}' requested but no display is available",
            selector
        );
        return;
    };

    let monitors = display.monitors();
    if let Some(monitor) = find_monitor_by_selector(&display, selector) {
        window.set_monitor(&monitor);
        return;
    }

    log_missing_output(selector, monitors.n_items());
}

fn find_monitor_by_selector(
    display: &gtk4::gdk::Display,
    selector: &str,
) -> Option<gtk4::gdk::Monitor> {
    let monitors = display.monitors();

    if let Ok(index) = selector.parse::<u32>() {
        return monitors
            .item(index)
            .and_then(|obj| obj.downcast::<gtk4::gdk::Monitor>().ok());
    }

    let sel = selector.to_lowercase();
    for i in 0..monitors.n_items() {
        let Some(obj) = monitors.item(i) else {
            continue;
        };
        let Ok(monitor) = obj.downcast::<gtk4::gdk::Monitor>() else {
            continue;
        };

        if monitor_matches(&monitor, &sel) {
            return Some(monitor);
        }
    }
    None
}

fn monitor_matches(monitor: &gtk4::gdk::Monitor, selector: &str) -> bool {
    monitor
        .connector()
        .map(|v| v.to_lowercase() == selector)
        .unwrap_or(false)
        || monitor
            .description()
            .map(|v| v.to_lowercase().contains(selector))
            .unwrap_or(false)
        || monitor
            .model()
            .map(|v| v.to_lowercase().contains(selector))
            .unwrap_or(false)
        || monitor
            .manufacturer()
            .map(|v| v.to_lowercase().contains(selector))
            .unwrap_or(false)
}

fn log_missing_output(selector: &str, monitor_count: u32) {
    if selector.parse::<u32>().is_ok() {
        log::warn!(
            "window output index '{}' not found ({} monitors available)",
            selector,
            monitor_count
        );
        return;
    }
    log::warn!(
        "window output '{}' not found by index/connector/name",
        selector
    );
}
