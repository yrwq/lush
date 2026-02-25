use std::process::Command;

use gtk4::prelude::*;
use gtk4::{Button, EventControllerScroll, EventControllerScrollFlags, GestureClick, Widget};

use crate::config::{ClickAction, ClickBindings, LoadedConfig};
use crate::runtime::signal_bus::SignalBus;

pub fn wire_button_click(
    button: &Button,
    on_click: &ClickBindings<ClickAction>,
    loaded: &LoadedConfig,
    bus: SignalBus,
) {
    if !on_click.has_any() {
        return;
    }

    if let Some(left) = on_click.left.clone() {
        let runtime = loaded.runtime.clone();
        let bus_for_left = bus.clone();
        button.connect_clicked(move |_| run_click_action(&left, &runtime, &bus_for_left));
    }

    let middle = on_click.middle.clone();
    let right = on_click.right.clone();
    let runtime = loaded.runtime.clone();
    let bus_for_gesture = bus.clone();
    let click = GestureClick::new();
    click.set_button(0);
    click.connect_pressed(move |gesture, _, _, _| match gesture.current_button() {
        2 => {
            if let Some(action) = middle.as_ref() {
                run_click_action(action, &runtime, &bus_for_gesture);
            }
        }
        3 => {
            if let Some(action) = right.as_ref() {
                run_click_action(action, &runtime, &bus_for_gesture);
            }
        }
        _ => {}
    });
    button.add_controller(click);

    wire_scroll(button, on_click, loaded, bus);
}

pub fn wire_gesture_click(
    widget: &impl IsA<Widget>,
    on_click: &ClickBindings<ClickAction>,
    loaded: &LoadedConfig,
    bus: SignalBus,
) {
    if !on_click.has_any() {
        return;
    }

    let left = on_click.left.clone();
    let middle = on_click.middle.clone();
    let right = on_click.right.clone();
    let runtime = loaded.runtime.clone();
    let bus_for_gesture = bus.clone();
    let click = GestureClick::new();
    click.set_button(0);
    click.connect_pressed(move |gesture, _, _, _| {
        let action = match gesture.current_button() {
            1 => left.as_ref(),
            2 => middle.as_ref(),
            3 => right.as_ref(),
            _ => None,
        };
        if let Some(action) = action {
            run_click_action(action, &runtime, &bus_for_gesture);
        }
    });
    widget.add_controller(click);

    wire_scroll(widget, on_click, loaded, bus);
}

fn wire_scroll(
    widget: &impl IsA<Widget>,
    on_click: &ClickBindings<ClickAction>,
    loaded: &LoadedConfig,
    bus: SignalBus,
) {
    let wheel_up = on_click.wheel_up.clone();
    let wheel_down = on_click.wheel_down.clone();
    if wheel_up.is_none() && wheel_down.is_none() {
        return;
    }

    let runtime = loaded.runtime.clone();
    let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
    scroll.connect_scroll(move |_, _, dy| {
        if dy < 0.0 {
            if let Some(action) = wheel_up.as_ref() {
                run_click_action(action, &runtime, &bus);
            }
            return glib::Propagation::Stop;
        }
        if dy > 0.0 {
            if let Some(action) = wheel_down.as_ref() {
                run_click_action(action, &runtime, &bus);
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    widget.add_controller(scroll);
}

fn run_click_action(
    action: &ClickAction,
    runtime: &std::rc::Rc<crate::runtime::lua_runtime::LuaRuntime>,
    bus: &SignalBus,
) {
    match action {
        ClickAction::Shell(command) => run_shell_or_special_action(command, bus),
        ClickAction::LuaCallback(id) => {
            if let Err(err) = runtime.invoke_click_callback(*id) {
                log::warn!("lua on_click callback failed: {}", err);
            }
        }
    }
}

fn run_shell_or_special_action(command: &str, bus: &SignalBus) {
    if let Some(index) = parse_notification_delete_action(command) {
        bus.set("notification.history_delete", &index.to_string());
        return;
    }
    if is_notification_clear_action(command) {
        bus.set("notification.history_clear", "1");
        return;
    }

    if let Err(err) = Command::new("sh").arg("-c").arg(command).spawn() {
        log::warn!("failed to run on_click command '{}': {}", command, err);
    }
}

fn parse_notification_delete_action(command: &str) -> Option<usize> {
    const PREFIX: &str = "lush.notifications.delete:";
    if let Some(value) = command.strip_prefix(PREFIX) {
        return value.trim().parse::<usize>().ok().filter(|v| *v > 0);
    }
    None
}

fn is_notification_clear_action(command: &str) -> bool {
    command.trim() == "lush.notifications.clear"
}
