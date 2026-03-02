use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};
use gtk4_layer_shell::{Edge, Layer, LayerShell, is_supported};

use crate::config::{AppConfig, LoadedConfig, WidgetConfig, WindowConfig};
use crate::runtime::notifications;
use crate::runtime::signal_bus::SignalBus;
use crate::ui::monitor;
use crate::ui::style::StyleManager;
use crate::ui::widgets::build_widget;

use super::control;
use super::{UiSession, WindowRegistry};

pub fn build_windows(app: &Application, cfg: &LoadedConfig) -> UiSession {
    let style = StyleManager::new(cfg.app.css.as_deref());
    log::info!(
        "building {} window(s), layer-shell supported={}",
        cfg.app.windows.len(),
        is_supported()
    );

    let usage = detect_integration_usage(cfg);
    log_integration_summary(&usage);

    if usage.needs_audio_provider
        && let Err(err) = cfg.runtime.data_use("audio", None, None, None, None)
    {
        log::warn!("failed to auto-start audio provider: {}", err);
    }

    let bus = SignalBus::default();
    control::wire_lua_signal_dispatch(&bus, cfg.runtime.clone());
    let windows: WindowRegistry = Rc::new(RefCell::new(HashMap::new()));
    control::wire_app_commands(cfg.runtime.clone(), &bus, &windows);

    for window_cfg in &cfg.app.windows {
        let window = build_window(app, window_cfg, &bus, &windows, cfg);
        control::register_window_if_named(&window, window_cfg, &windows);
        control::apply_initial_visibility(&window, window_cfg);
        control::publish_window_visibility(
            cfg.runtime.as_ref(),
            &bus,
            window_cfg.name.as_deref(),
            window.is_visible(),
        );
    }

    cfg.runtime.attach_signal_bus(bus.clone());
    if usage.needs_notifications_runtime {
        notifications::start(bus.clone());
    }

    control::present_named_windows(&cfg.app.windows, &windows, cfg.runtime.as_ref(), &bus);

    UiSession {
        runtime: cfg.runtime.clone(),
        bus,
        windows,
        style,
    }
}

pub fn reconfigure_windows(app: &Application, session: &mut UiSession, app_cfg: &AppConfig) {
    if app_cfg.windows.iter().any(|w| w.name.is_none()) {
        log::warn!("reconfigure: unnamed windows are ignored; use named windows for live config");
    }

    session.style.set_path(app_cfg.css.as_deref());
    if let Err(err) = session.style.reload() {
        log::warn!("reconfigure: css reload failed: {}", err);
    }

    let desired_named: HashMap<String, &WindowConfig> = app_cfg
        .windows
        .iter()
        .filter_map(|cfg| cfg.name.as_ref().map(|name| (name.clone(), cfg)))
        .collect();

    let existing_names: Vec<String> = session.windows.borrow().keys().cloned().collect();
    for name in existing_names {
        if desired_named.contains_key(&name) {
            continue;
        }
        if let Some(window) = session.windows.borrow_mut().remove(&name) {
            window.close();
            control::publish_window_visibility(
                session.runtime.as_ref(),
                &session.bus,
                Some(&name),
                false,
            );
        }
    }

    let loaded_for_new = LoadedConfig {
        app: app_cfg.clone(),
        runtime: session.runtime.clone(),
    };

    for (name, window_cfg) in desired_named {
        if let Some(window) = session.windows.borrow().get(&name).cloned() {
            apply_window_config(&window, window_cfg);
            control::set_named_window_visible(
                session.runtime.as_ref(),
                &session.bus,
                &session.windows,
                &name,
                window_cfg.visible,
            );
            continue;
        }

        let window = build_window(
            app,
            window_cfg,
            &session.bus,
            &session.windows,
            &loaded_for_new,
        );
        control::register_window_if_named(&window, window_cfg, &session.windows);
        control::set_named_window_visible(
            session.runtime.as_ref(),
            &session.bus,
            &session.windows,
            &name,
            window_cfg.visible,
        );
    }
}

#[derive(Default)]
struct IntegrationUsage {
    needs_audio_provider: bool,
    needs_notifications_runtime: bool,
}

fn log_integration_summary(usage: &IntegrationUsage) {
    let audio_provider = if usage.needs_audio_provider {
        "enabled"
    } else {
        "disabled"
    };
    let notifications_runtime = if usage.needs_notifications_runtime {
        "enabled"
    } else {
        "disabled"
    };

    log::info!(
        "integrations: audio-provider={}, notifications-runtime={}",
        audio_provider,
        notifications_runtime,
    );
}

fn detect_integration_usage(cfg: &LoadedConfig) -> IntegrationUsage {
    let mut usage = IntegrationUsage::default();
    for window in &cfg.app.windows {
        collect_widget_usage(&window.root, &mut usage);
    }
    usage
}

fn collect_widget_usage(cfg: &WidgetConfig, usage: &mut IntegrationUsage) {
    collect_signal_usage(cfg.base.visible_bind.as_deref(), usage);
    collect_signal_usage(cfg.base.class_bind.as_deref(), usage);

    match &cfg.props {
        crate::config::WidgetProps::Revealer(props) => {
            collect_signal_usage(props.reveal_bind.as_deref(), usage);
        }
        crate::config::WidgetProps::List(props) => {
            collect_signal_usage(props.bind.as_deref(), usage);
        }
        crate::config::WidgetProps::Label(props) => {
            collect_signal_usage(props.bind.as_deref(), usage);
            for bind in props.binds.values() {
                collect_signal_usage(Some(bind.as_str()), usage);
            }
        }
        crate::config::WidgetProps::Button(props) => {
            collect_signal_usage(props.bind.as_deref(), usage);
        }
        crate::config::WidgetProps::Clock(props) => {
            collect_signal_usage(props.bind.as_deref(), usage);
        }
        crate::config::WidgetProps::Image(props) => {
            collect_signal_usage(props.bind.as_deref(), usage);
        }
        crate::config::WidgetProps::Progress(props) => {
            collect_signal_usage(props.bind.as_deref(), usage);
        }
        crate::config::WidgetProps::Slider(props) => {
            collect_signal_usage(props.bind.as_deref(), usage);
        }
        crate::config::WidgetProps::HBox(_)
        | crate::config::WidgetProps::VBox(_)
        | crate::config::WidgetProps::CenterBox(_)
        | crate::config::WidgetProps::Scroll(_)
        | crate::config::WidgetProps::Overlay(_)
        | crate::config::WidgetProps::Popover(_)
        | crate::config::WidgetProps::Workspaces(_)
        | crate::config::WidgetProps::Dock(_)
        | crate::config::WidgetProps::Tray(_) => {}
    }

    for child in &cfg.base.children {
        collect_widget_usage(child, usage);
    }
}

fn collect_signal_usage(signal_name: Option<&str>, usage: &mut IntegrationUsage) {
    let Some(signal_name) = signal_name.map(str::trim).filter(|v| !v.is_empty()) else {
        return;
    };

    if signal_name.starts_with("data.audio.") {
        usage.needs_audio_provider = true;
    }

    if signal_name.starts_with("notification.") {
        usage.needs_notifications_runtime = true;
    }
}

fn build_window(
    app: &Application,
    window_cfg: &WindowConfig,
    bus: &SignalBus,
    windows: &WindowRegistry,
    loaded: &LoadedConfig,
) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(window_cfg.name.as_deref().unwrap_or("lush"))
        .build();
    window.add_css_class("lush-window");

    apply_window_config(&window, window_cfg);

    let root = build_window_root(window_cfg, bus, windows, loaded);
    window.set_child(Some(&root));
    window
}

fn apply_window_config(window: &ApplicationWindow, window_cfg: &WindowConfig) {
    window.set_title(Some(window_cfg.name.as_deref().unwrap_or("lush")));
    if is_supported() {
        configure_layer_shell_window(window, window_cfg);
    } else {
        configure_fallback_window(window, window_cfg);
    }
    if let Some(root) = window.child() {
        root.set_width_request(window_cfg.width.unwrap_or(-1));
        root.set_height_request(window_cfg.height.unwrap_or(-1));
    }
}

fn build_window_root(
    window_cfg: &WindowConfig,
    bus: &SignalBus,
    windows: &WindowRegistry,
    loaded: &LoadedConfig,
) -> gtk4::Widget {
    let widget_output_selector =
        monitor::resolve_widget_output_selector(window_cfg.output.as_deref());
    let root = build_widget(
        &window_cfg.root,
        bus,
        windows,
        loaded,
        widget_output_selector.as_deref(),
    );
    if let Some(v) = window_cfg.width {
        root.set_width_request(v);
    }
    if let Some(v) = window_cfg.height {
        root.set_height_request(v);
    }
    root
}

fn configure_layer_shell_window(window: &ApplicationWindow, window_cfg: &WindowConfig) {
    log::info!(
        "window '{}' using layer-shell",
        window_cfg.name.as_deref().unwrap_or("unnamed")
    );

    window.init_layer_shell();
    window.set_layer(parse_layer(&window_cfg.layer));
    monitor::apply_output_target(window, window_cfg.output.as_deref());

    apply_anchors(window, &window_cfg.anchors);

    if window_cfg.exclusive {
        window.set_exclusive_zone(exclusive_zone_for(window_cfg));
    } else {
        window.set_exclusive_zone(0);
    }

    apply_margins(window, window_cfg);
}

fn configure_fallback_window(window: &ApplicationWindow, window_cfg: &WindowConfig) {
    window.set_default_size(
        window_cfg.width.unwrap_or(700),
        window_cfg.height.unwrap_or(120),
    );
    log::info!(
        "window '{}' using normal GTK fallback",
        window_cfg.name.as_deref().unwrap_or("unnamed")
    );
}

fn apply_anchors(window: &ApplicationWindow, anchors: &[String]) {
    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        window.set_anchor(edge, false);
    }

    for edge in anchors {
        match edge.as_str() {
            "top" => window.set_anchor(Edge::Top, true),
            "bottom" => window.set_anchor(Edge::Bottom, true),
            "left" => window.set_anchor(Edge::Left, true),
            "right" => window.set_anchor(Edge::Right, true),
            other => log::warn!("unknown anchor '{}'", other),
        }
    }
}

fn apply_margins(window: &ApplicationWindow, window_cfg: &WindowConfig) {
    window.set_margin(Edge::Top, window_cfg.margin_top);
    window.set_margin(Edge::Bottom, window_cfg.margin_bottom);
    window.set_margin(Edge::Left, window_cfg.margin_left);
    window.set_margin(Edge::Right, window_cfg.margin_right);
}

fn parse_layer(layer: &str) -> Layer {
    match layer {
        "background" => Layer::Background,
        "bottom" => Layer::Bottom,
        "overlay" => Layer::Overlay,
        _ => Layer::Top,
    }
}

fn exclusive_zone_for(window_cfg: &WindowConfig) -> i32 {
    let has_top = window_cfg.anchors.iter().any(|a| a == "top");
    let has_bottom = window_cfg.anchors.iter().any(|a| a == "bottom");
    let has_left = window_cfg.anchors.iter().any(|a| a == "left");
    let has_right = window_cfg.anchors.iter().any(|a| a == "right");

    if (has_left || has_right) && (has_top == has_bottom) {
        return window_cfg.width.unwrap_or(30);
    }

    window_cfg.height.unwrap_or(30)
}
