use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{EventControllerScroll, EventControllerScrollFlags, Orientation, Scale, Widget};

use crate::config::{WidgetConfig, WidgetProps};
use crate::ui::signal_watch::watch_signal;
use crate::ui::widgets::core::bindings::initial_value_from_bind_or;
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::finalize_widget;

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Slider(props) = &cfg.props else {
        unreachable!("slider builder received non-slider props");
    };

    let min = props.min.unwrap_or(0.0);
    let max = props.max.unwrap_or(100.0);
    let step = props.step.unwrap_or(1.0).abs().max(f64::EPSILON);
    let scroll_step = props.scroll_step.unwrap_or(step).abs().max(f64::EPSILON);
    let fallback = props.value.unwrap_or(min);
    let initial_value = parse_f64_or(
        &initial_value_from_bind_or(props.bind.as_deref(), ctx.bus, fallback.to_string()),
        fallback,
    );
    let orientation = parse_orientation(props.orientation.as_deref());
    let digits = props.digits.unwrap_or(0).clamp(0, 6);

    let slider = Scale::with_range(orientation, min, max.max(min + f64::EPSILON), step);
    slider.set_draw_value(props.draw_value.unwrap_or(false));
    slider.set_digits(digits);
    if let Some(inverted) = props.inverted {
        slider.set_inverted(inverted);
    }
    slider.set_value(clamp_value(initial_value, min, max));

    let syncing_from_bus = Rc::new(Cell::new(false));

    if let Some(bind_name) = props.bind.clone() {
        let bus_for_change = ctx.bus.clone();
        let bind_for_change = bind_name.clone();
        let syncing_for_change = syncing_from_bus.clone();
        slider.connect_value_changed(move |scale| {
            if syncing_for_change.get() {
                return;
            }
            let snapped = snap_to_step(scale.value(), min, max, step);
            if (snapped - scale.value()).abs() > f64::EPSILON {
                syncing_for_change.set(true);
                scale.set_value(snapped);
                syncing_for_change.set(false);
            }
            let value = format_numeric(snapped, digits);
            bus_for_change.set(&bind_for_change, &value);
        });

        let weak = slider.downgrade();
        let syncing_for_watch = syncing_from_bus.clone();
        watch_signal(ctx.bus, bind_name, move |value| {
            if let Some(slider) = weak.upgrade() {
                let current = slider.value();
                let parsed = parse_f64_or(value, current);
                syncing_for_watch.set(true);
                slider.set_value(clamp_value(parsed, min, max));
                syncing_for_watch.set(false);
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
    }

    let scroll = EventControllerScroll::new(
        EventControllerScrollFlags::VERTICAL
            | EventControllerScrollFlags::HORIZONTAL
            | EventControllerScrollFlags::DISCRETE,
    );
    let weak = slider.downgrade();
    scroll.connect_scroll(move |_, dx, dy| {
        let Some(slider) = weak.upgrade() else {
            return glib::Propagation::Proceed;
        };
        let dominant = if dy.abs() >= dx.abs() { dy } else { dx };
        if dominant == 0.0 {
            return glib::Propagation::Proceed;
        }

        let current = slider.value();
        let delta = if dominant < 0.0 {
            scroll_step
        } else {
            -scroll_step
        };
        slider.set_value(clamp_value(current + delta, min, max));
        glib::Propagation::Stop
    });
    slider.add_controller(scroll);

    let widget: Widget = slider.upcast();
    finalize_widget(&widget, cfg, ctx.bus, false);
    widget
}

fn parse_orientation(raw: Option<&str>) -> Orientation {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(value) if value == "vertical" => Orientation::Vertical,
        _ => Orientation::Horizontal,
    }
}

fn parse_f64_or(value: &str, fallback: f64) -> f64 {
    value.trim().parse::<f64>().ok().unwrap_or(fallback)
}

fn clamp_value(value: f64, min: f64, max: f64) -> f64 {
    if max <= min {
        return min;
    }
    value.clamp(min, max)
}

fn format_numeric(value: f64, digits: i32) -> String {
    let digits = digits.clamp(0, 6) as usize;
    if digits == 0 {
        return format!("{}", value.round() as i64);
    }
    let text = format!("{:.*}", digits, value);
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn snap_to_step(value: f64, min: f64, max: f64, step: f64) -> f64 {
    let step = step.abs().max(f64::EPSILON);
    let clamped = clamp_value(value, min, max);
    let snapped = min + ((clamped - min) / step).round() * step;
    clamp_value(snapped, min, max)
}
