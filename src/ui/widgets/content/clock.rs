use std::cell::RefCell;
use std::rc::Rc;

use chrono::Local;
use glib::timeout_add_seconds_local;
use gtk4::prelude::*;
use gtk4::{Box as GBox, Label, Orientation, Widget};

use crate::config::{ClockProps, WidgetConfig, WidgetProps};
use crate::runtime::signal_bus::SignalBus;
use crate::ui::widgets::core::bindings::{initial_state_from_class_bind, watch_class_state};
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::{finalize_widget, format_text};
use crate::ui::widgets::core::format::render as render_format;

const DEFAULT_CLOCK_FORMAT: &str = "%a %d.%b %H:%M:%S";
const DEFAULT_CLOCK_INTERVAL_SECONDS: u32 = 1;

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Clock(props) = &cfg.props else {
        unreachable!("clock builder received non-clock props");
    };

    let time_format = props
        .format
        .clone()
        .unwrap_or_else(|| DEFAULT_CLOCK_FORMAT.to_string());
    let interval = props
        .interval
        .map(|seconds| seconds as u32)
        .unwrap_or(DEFAULT_CLOCK_INTERVAL_SECONDS);

    let container = GBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(props.spacing.unwrap_or(6))
        .build();

    let time_label = Label::new(None);
    time_label.add_css_class("clock-time");
    let current_value = Rc::new(RefCell::new(String::new()));
    let current_state = Rc::new(RefCell::new(initial_state_from_class_bind(
        cfg.base.class_bind.as_deref(),
        ctx.bus,
    )));
    let initial_value = now_string(&time_format);
    *current_value.borrow_mut() = initial_value.clone();
    apply_clock_text(&time_label, cfg, &initial_value, &current_state.borrow());
    publish_bound_signal(ctx.bus, props.bind.as_deref(), &initial_value);

    container.append(&time_label);
    start_clock_timer(
        &time_label,
        interval,
        ClockTimerCtx {
            cfg: cfg.clone(),
            time_format: time_format.clone(),
            bind_name: props.bind.clone(),
            bus: ctx.bus.clone(),
            value: current_value.clone(),
            state: current_state.clone(),
        },
    );
    watch_class_state(
        &time_label,
        cfg,
        cfg.base.class_bind.clone(),
        ctx.bus,
        current_value.clone(),
        current_state.clone(),
        apply_clock_text,
    );

    let widget: Widget = container.upcast();
    finalize_widget(&widget, cfg, ctx.bus, false);
    widget
}

fn now_string(format: &str) -> String {
    Local::now().format(format).to_string()
}

fn publish_bound_signal(bus: &SignalBus, bind_name: Option<&str>, value: &str) {
    if let Some(signal_name) = bind_name {
        bus.set(signal_name, value);
    }
}

fn start_clock_timer(label: &Label, interval_seconds: u32, timer_ctx: ClockTimerCtx) {
    let weak = label.downgrade();
    timeout_add_seconds_local(interval_seconds, move || {
        let Some(label) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let next_value = now_string(&timer_ctx.time_format);
        *timer_ctx.value.borrow_mut() = next_value.clone();
        apply_clock_text(
            &label,
            &timer_ctx.cfg,
            &next_value,
            &timer_ctx.state.borrow(),
        );
        publish_bound_signal(&timer_ctx.bus, timer_ctx.bind_name.as_deref(), &next_value);
        glib::ControlFlow::Continue
    });
}

struct ClockTimerCtx {
    cfg: WidgetConfig,
    time_format: String,
    bind_name: Option<String>,
    bus: SignalBus,
    value: Rc<RefCell<String>>,
    state: Rc<RefCell<String>>,
}

fn apply_clock_text(label: &Label, cfg: &WidgetConfig, value: &str, state: &str) {
    let WidgetProps::Clock(ClockProps {
        display_format,
        format_states,
        angle,
        ..
    }) = &cfg.props
    else {
        return;
    };

    let rendered = render_format(
        display_format.as_deref().or(Some("{value}")),
        format_states,
        state,
        &[
            ("value", value.to_string()),
            ("time", value.to_string()),
            ("state", state.to_string()),
        ],
    );
    label.set_text(&format_text(&rendered, *angle));
}
