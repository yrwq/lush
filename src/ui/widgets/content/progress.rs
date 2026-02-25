use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, LevelBar, Orientation, Widget};

use crate::config::{WidgetConfig, WidgetProps};
use crate::ui::signal_watch::watch_signal;
use crate::ui::widgets::core::bindings::{
    initial_state_from_class_bind, initial_value_from_bind_or,
};
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::finalize_widget;

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Progress(props) = &cfg.props else {
        unreachable!("progress builder received non-progress props");
    };

    let min = props.min.unwrap_or(0.0);
    let fallback_value = props.value.unwrap_or(min).to_string();
    let initial_value = initial_value_from_bind_or(props.bind.as_deref(), ctx.bus, fallback_value);
    let initial_state = initial_state_from_class_bind(cfg.base.class_bind.as_deref(), ctx.bus);

    let bar = LevelBar::new();
    bar.set_mode(gtk4::LevelBarMode::Continuous);
    bar.set_orientation(Orientation::Horizontal);
    bar.set_min_value(min);
    bar.set_max_value(props.max.unwrap_or(100.0));

    if cfg.base.width.is_some() {
        if cfg.base.hexpand.is_none() {
            bar.set_hexpand(false);
        }
        if cfg.base.halign.is_none() {
            bar.set_halign(Align::Start);
        }
    }

    if props.inverted.unwrap_or(false) {
        bar.add_css_class("inverted");
    }

    let root: Widget = bar.clone().upcast();

    let current_value = Rc::new(RefCell::new(initial_value));
    let current_state = Rc::new(RefCell::new(initial_state));
    apply_progress(&bar, cfg, &current_value.borrow(), &current_state.borrow());

    if let Some(bind_name) = props.bind.clone() {
        let bar_weak = bar.downgrade();
        let cfg_for_watch = cfg.clone();
        let value_for_watch = current_value.clone();
        let state_for_watch = current_state.clone();
        watch_signal(ctx.bus, bind_name, move |value| {
            *value_for_watch.borrow_mut() = value.to_string();
            if let Some(bar) = bar_weak.upgrade() {
                apply_progress(
                    &bar,
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

    if let Some(class_bind) = cfg.base.class_bind.clone() {
        let bar_weak = bar.downgrade();
        let cfg_for_watch = cfg.clone();
        let value_for_watch = current_value.clone();
        let state_for_watch = current_state.clone();
        watch_signal(ctx.bus, class_bind, move |state| {
            *state_for_watch.borrow_mut() = state.to_string();
            if let Some(bar) = bar_weak.upgrade() {
                apply_progress(
                    &bar,
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

    finalize_widget(&root, cfg, ctx.bus, false);
    root
}

fn apply_progress(bar: &LevelBar, cfg: &WidgetConfig, value: &str, _state: &str) {
    let WidgetProps::Progress(props) = &cfg.props else {
        return;
    };

    let min = props.min.unwrap_or(0.0);
    let max = props.max.unwrap_or(100.0);
    let fallback = props.value.unwrap_or(min);
    let raw_value = parse_f64_or(value, fallback);
    let clamped_value = clamp_value(raw_value, min, max);

    bar.set_min_value(min);
    bar.set_max_value(max.max(min + f64::EPSILON));
    bar.set_value(clamped_value);
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
