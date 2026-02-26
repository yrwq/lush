use std::rc::Rc;

use gtk4::ApplicationWindow;
use gtk4::prelude::*;

use crate::config::WindowConfig;
use crate::runtime::lua_runtime::{AppCommand, LuaRuntime};
use crate::runtime::signal_bus::SignalBus;

use super::WindowRegistry;

pub(super) fn wire_app_commands(
    runtime: Rc<LuaRuntime>,
    bus: &SignalBus,
    windows: &WindowRegistry,
) {
    let windows = windows.clone();
    let bus = bus.clone();
    let runtime_weak = Rc::downgrade(&runtime);
    runtime.subscribe_app_commands(move |cmd| {
        let Some(runtime) = runtime_weak.upgrade() else {
            return false;
        };
        match cmd {
            AppCommand::Open(name) => open_named_window(runtime.as_ref(), &bus, &windows, name),
            AppCommand::Close(name) => close_named_window(runtime.as_ref(), &bus, &windows, name),
            AppCommand::Toggle(name) => toggle_named_window(runtime.as_ref(), &bus, &windows, name),
            AppCommand::SetVisible(name, visible) => {
                set_named_window_visible(runtime.as_ref(), &bus, &windows, name, *visible)
            }
        }
        true
    });
}

pub(super) fn wire_lua_signal_dispatch(bus: &SignalBus, runtime: Rc<LuaRuntime>) {
    let runtime_weak = Rc::downgrade(&runtime);
    bus.subscribe(move |event| {
        let Some(runtime) = runtime_weak.upgrade() else {
            return false;
        };
        if let Err(err) = runtime.dispatch_signal(&event.name, &event.value) {
            log::warn!("lush signal callback failed: {}", err);
        }
        true
    });
}

pub(super) fn register_window_if_named(
    window: &ApplicationWindow,
    cfg: &WindowConfig,
    windows: &WindowRegistry,
) {
    if let Some(name) = &cfg.name {
        windows.borrow_mut().insert(name.clone(), window.clone());
        log::info!("registered window '{}'", name);
    }
}

pub(super) fn apply_initial_visibility(window: &ApplicationWindow, cfg: &WindowConfig) {
    if cfg.name.is_some() {
        return;
    }

    if cfg.visible {
        window.present();
        log::info!("presented unnamed window");
    } else {
        window.set_visible(false);
        log::info!("kept unnamed window hidden");
    }
}

pub(super) fn present_named_windows(
    configs: &[WindowConfig],
    windows: &WindowRegistry,
    runtime: &LuaRuntime,
    bus: &SignalBus,
) {
    for cfg in configs {
        let Some(name) = &cfg.name else {
            continue;
        };

        if let Some(window) = windows.borrow().get(name) {
            set_window_visible(runtime, bus, window, name, cfg.visible);
            if cfg.visible {
                log::info!("presented window '{}'", name);
            } else {
                log::info!("kept window '{}' hidden", name);
            }
        }
    }
}

pub(super) fn publish_window_visibility(
    runtime: &LuaRuntime,
    bus: &SignalBus,
    name: Option<&str>,
    visible: bool,
) {
    let Some(name) = name else {
        return;
    };
    runtime.set_window_visible(name, visible);
    let key = format!("window.{}.visible", name);
    bus.set(&key, if visible { "1" } else { "0" });
}

fn open_named_window(runtime: &LuaRuntime, bus: &SignalBus, windows: &WindowRegistry, name: &str) {
    set_named_window_visible(runtime, bus, windows, name, true);
}

fn close_named_window(runtime: &LuaRuntime, bus: &SignalBus, windows: &WindowRegistry, name: &str) {
    set_named_window_visible(runtime, bus, windows, name, false);
}

fn toggle_named_window(
    runtime: &LuaRuntime,
    bus: &SignalBus,
    windows: &WindowRegistry,
    name: &str,
) {
    if let Some(window) = windows.borrow().get(name) {
        let visible = !window.is_visible();
        set_window_visible(runtime, bus, window, name, visible);
    }
}

fn set_named_window_visible(
    runtime: &LuaRuntime,
    bus: &SignalBus,
    windows: &WindowRegistry,
    name: &str,
    visible: bool,
) {
    if let Some(window) = windows.borrow().get(name) {
        set_window_visible(runtime, bus, window, name, visible);
    }
}

fn set_window_visible(
    runtime: &LuaRuntime,
    bus: &SignalBus,
    window: &ApplicationWindow,
    name: &str,
    visible: bool,
) {
    if visible {
        window.present();
    } else {
        window.set_visible(false);
    }
    publish_window_visibility(runtime, bus, Some(name), visible);
}
