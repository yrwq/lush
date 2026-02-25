use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use gtk4::{Box as GBox, Label, Orientation, Widget};

use crate::config::{WidgetConfig, WidgetProps};
use crate::ui::widgets::core::bindings::{
    initial_state_from_class_bind, initial_value_from_bind_or,
};
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::clicks::wire_gesture_click;
use crate::ui::widgets::core::common::finalize_widget;

mod render;
mod watch;

use self::render::{apply_segmented, apply_single_markup};
use self::watch::{LabelWatchCtx, install_label_watchers};

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Label(props) = &cfg.props else {
        unreachable!("label builder received non-label props");
    };

    let base_text = props.text.clone().unwrap_or_default();
    let current_value = Rc::new(RefCell::new(initial_value_from_bind_or(
        props.bind.as_deref(),
        ctx.bus,
        base_text,
    )));
    let current_state = Rc::new(RefCell::new(initial_state_from_class_bind(
        cfg.base.class_bind.as_deref(),
        ctx.bus,
    )));
    let named_tokens = Rc::new(RefCell::new(
        props
            .binds
            .iter()
            .map(|(token, signal)| (token.clone(), ctx.bus.get(signal).unwrap_or_default()))
            .collect::<HashMap<String, String>>(),
    ));

    let has_fragment_classes = props.rules.iter().any(|rule| {
        rule.style
            .class_name
            .as_deref()
            .map(str::trim)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    });

    if has_fragment_classes {
        build_segmented(cfg, ctx, props, current_value, current_state, named_tokens)
    } else {
        build_single_markup(cfg, ctx, props, current_value, current_state, named_tokens)
    }
}

fn build_single_markup(
    cfg: &WidgetConfig,
    ctx: &WidgetBuildCtx<'_>,
    props: &crate::config::LabelProps,
    current_value: Rc<RefCell<String>>,
    current_state: Rc<RefCell<String>>,
    named_tokens: Rc<RefCell<HashMap<String, String>>>,
) -> Widget {
    let label = Label::new(None);
    label.set_use_markup(true);
    if let Some(v) = props.max_chars.filter(|v| *v > 0) {
        label.set_max_width_chars(v);
    }
    if let Some(mode) = parse_ellipsize(props.ellipsize.as_deref()) {
        label.set_wrap(false);
        label.set_ellipsize(mode);
    }
    wire_gesture_click(&label, &props.on_click, ctx.loaded, ctx.bus.clone());

    let last_markup = Rc::new(RefCell::new(String::new()));
    apply_single_markup(
        &label,
        cfg,
        &current_value.borrow(),
        &current_state.borrow(),
        &named_tokens.borrow(),
        &last_markup,
    );

    let last_markup_for_watch = last_markup.clone();
    install_label_watchers(
        &label,
        cfg,
        props,
        LabelWatchCtx {
            bus: ctx.bus.clone(),
            current_value,
            current_state,
            named_tokens,
        },
        move |widget, cfg, value, state, named_tokens| {
            apply_single_markup(
                widget,
                cfg,
                value,
                state,
                named_tokens,
                &last_markup_for_watch,
            );
        },
    );

    let widget: Widget = label.upcast();
    finalize_widget(&widget, cfg, ctx.bus, props.on_click.has_any());
    widget
}

fn build_segmented(
    cfg: &WidgetConfig,
    ctx: &WidgetBuildCtx<'_>,
    props: &crate::config::LabelProps,
    current_value: Rc<RefCell<String>>,
    current_state: Rc<RefCell<String>>,
    named_tokens: Rc<RefCell<HashMap<String, String>>>,
) -> Widget {
    let row = GBox::new(Orientation::Horizontal, 0);
    wire_gesture_click(&row, &props.on_click, ctx.loaded, ctx.bus.clone());

    let last_signature = Rc::new(RefCell::new(String::new()));
    apply_segmented(
        &row,
        cfg,
        &current_value.borrow(),
        &current_state.borrow(),
        &named_tokens.borrow(),
        &last_signature,
    );

    let last_signature_for_watch = last_signature.clone();
    install_label_watchers(
        &row,
        cfg,
        props,
        LabelWatchCtx {
            bus: ctx.bus.clone(),
            current_value,
            current_state,
            named_tokens,
        },
        move |widget, cfg, value, state, named_tokens| {
            apply_segmented(
                widget,
                cfg,
                value,
                state,
                named_tokens,
                &last_signature_for_watch,
            );
        },
    );

    let widget: Widget = row.upcast();
    finalize_widget(&widget, cfg, ctx.bus, props.on_click.has_any());
    widget
}

fn parse_ellipsize(value: Option<&str>) -> Option<EllipsizeMode> {
    match value.unwrap_or("none") {
        "start" => Some(EllipsizeMode::Start),
        "middle" => Some(EllipsizeMode::Middle),
        "end" => Some(EllipsizeMode::End),
        _ => None,
    }
}
