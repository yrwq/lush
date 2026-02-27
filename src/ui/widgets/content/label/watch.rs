use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk4::Widget;
use gtk4::prelude::*;

use crate::config::{LabelProps, WidgetConfig};
use crate::runtime::signal_bus::SignalBus;
use crate::ui::signal_watch::{watch_signal, window_is_visible};

pub struct LabelWatchCtx {
    pub bus: SignalBus,
    pub current_value: Rc<RefCell<String>>,
    pub current_state: Rc<RefCell<String>>,
    pub named_tokens: Rc<RefCell<HashMap<String, String>>>,
}

pub fn install_label_watchers<W, F>(
    widget: &W,
    cfg: &WidgetConfig,
    props: &LabelProps,
    watch_ctx: LabelWatchCtx,
    apply: F,
) where
    W: IsA<Widget> + Clone + 'static,
    F: Fn(&W, &WidgetConfig, &str, &str, &HashMap<String, String>) + Clone + 'static,
{
    let LabelWatchCtx {
        bus,
        current_value,
        current_state,
        named_tokens,
    } = watch_ctx;

    if let Some(bind_name) = props.bind.clone() {
        let weak = widget.downgrade();
        let cfg_for_watch = cfg.clone();
        let current_value_for_watch = current_value.clone();
        let current_state_for_watch = current_state.clone();
        let named_tokens_for_watch = named_tokens.clone();
        let apply_for_watch = apply.clone();
        watch_signal(&bus, bind_name, move |value| {
            *current_value_for_watch.borrow_mut() = value.to_string();
            if let Some(widget) = weak.upgrade() {
                if !window_is_visible(&widget) {
                    return glib::ControlFlow::Continue;
                }
                apply_for_watch(
                    &widget,
                    &cfg_for_watch,
                    &current_value_for_watch.borrow(),
                    &current_state_for_watch.borrow(),
                    &named_tokens_for_watch.borrow(),
                );
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
    }

    if let Some(class_bind) = cfg.base.class_bind.clone() {
        let weak = widget.downgrade();
        let cfg_for_watch = cfg.clone();
        let current_value_for_watch = current_value.clone();
        let current_state_for_watch = current_state.clone();
        let named_tokens_for_watch = named_tokens.clone();
        let apply_for_watch = apply.clone();
        watch_signal(&bus, class_bind, move |state| {
            *current_state_for_watch.borrow_mut() = state.to_string();
            if let Some(widget) = weak.upgrade() {
                if !window_is_visible(&widget) {
                    return glib::ControlFlow::Continue;
                }
                apply_for_watch(
                    &widget,
                    &cfg_for_watch,
                    &current_value_for_watch.borrow(),
                    &current_state_for_watch.borrow(),
                    &named_tokens_for_watch.borrow(),
                );
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
    }

    for (token, signal) in props.binds.clone() {
        let weak = widget.downgrade();
        let cfg_for_watch = cfg.clone();
        let current_value_for_watch = current_value.clone();
        let current_state_for_watch = current_state.clone();
        let named_tokens_for_watch = named_tokens.clone();
        let apply_for_watch = apply.clone();
        watch_signal(&bus, signal, move |value| {
            named_tokens_for_watch
                .borrow_mut()
                .insert(token.clone(), value.to_string());
            if let Some(widget) = weak.upgrade() {
                if !window_is_visible(&widget) {
                    return glib::ControlFlow::Continue;
                }
                apply_for_watch(
                    &widget,
                    &cfg_for_watch,
                    &current_value_for_watch.borrow(),
                    &current_state_for_watch.borrow(),
                    &named_tokens_for_watch.borrow(),
                );
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
    }
}
