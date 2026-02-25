use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Button, Widget};

use crate::config::{ButtonProps, WidgetConfig, WidgetProps};
use crate::ui::widgets::core::bindings::{
    initial_state_from_class_bind, initial_value_from_bind_or, watch_bind_value, watch_class_state,
};
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::clicks::wire_button_click;
use crate::ui::widgets::core::common::{finalize_widget, format_text};
use crate::ui::widgets::core::format::render as render_format;

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Button(props) = &cfg.props else {
        unreachable!("button builder received non-button props");
    };

    let base_text = props.text.clone().unwrap_or_else(|| "button".to_string());
    let initial_value =
        initial_value_from_bind_or(props.bind.as_deref(), ctx.bus, base_text.clone());
    let initial_state = initial_state_from_class_bind(cfg.base.class_bind.as_deref(), ctx.bus);

    let button = Button::new();
    let current_value = Rc::new(RefCell::new(initial_value));
    let current_state = Rc::new(RefCell::new(initial_state));
    apply_button_label(
        &button,
        cfg,
        &current_value.borrow(),
        &current_state.borrow(),
    );

    watch_bind_value(
        &button,
        cfg,
        props.bind.clone(),
        ctx.bus,
        current_value.clone(),
        current_state.clone(),
        apply_button_label,
    );
    watch_class_state(
        &button,
        cfg,
        cfg.base.class_bind.clone(),
        ctx.bus,
        current_value.clone(),
        current_state.clone(),
        apply_button_label,
    );

    wire_button_click(&button, &props.on_click, ctx.loaded, ctx.bus.clone());

    let widget: Widget = button.upcast();
    finalize_widget(&widget, cfg, ctx.bus, props.on_click.has_any());
    widget
}

fn apply_button_label(button: &Button, cfg: &WidgetConfig, value: &str, state: &str) {
    let WidgetProps::Button(ButtonProps {
        text,
        format,
        format_states,
        angle,
        ..
    }) = &cfg.props
    else {
        return;
    };

    let text_value = text.clone().unwrap_or_else(|| "button".to_string());
    let rendered = render_format(
        format.as_deref().or(Some("{text}")),
        format_states,
        state,
        &[
            ("text", text_value),
            ("value", value.to_string()),
            ("state", state.to_string()),
        ],
    );
    button.set_label(&format_text(&rendered, *angle));
}
