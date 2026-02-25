use gtk4::prelude::*;
use gtk4::{Revealer as GRevealer, RevealerTransitionType, Widget};

use crate::config::{RevealerProps, WidgetConfig, WidgetProps};
use crate::ui::signal_watch::watch_signal;
use crate::ui::widgets::build_widget;
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::finalize_widget;

const DEFAULT_REVEAL_DURATION_MS: u32 = 250;

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Revealer(props) = &cfg.props else {
        unreachable!("revealer builder received non-revealer props");
    };

    let container = GRevealer::new();
    container.set_transition_duration(props.duration.unwrap_or(DEFAULT_REVEAL_DURATION_MS));
    container.set_transition_type(parse_transition(props.transition.as_deref()));

    if let Some(child) = cfg.base.children.first() {
        container.set_child(Some(&build_widget(
            child,
            ctx.bus,
            ctx.windows,
            ctx.loaded,
            ctx.default_output,
        )));
    }

    if cfg.base.children.len() > 1 {
        log::warn!(
            "revealer uses only first child as content; got {}",
            cfg.base.children.len()
        );
    }

    container.set_reveal_child(props.reveal.unwrap_or(true));
    wire_reveal_binding(&container, props, ctx);

    let widget: Widget = container.upcast();
    finalize_widget(&widget, cfg, ctx.bus, false);
    widget
}

fn wire_reveal_binding(revealer: &GRevealer, props: &RevealerProps, ctx: &WidgetBuildCtx<'_>) {
    let Some(signal_name) = props.reveal_bind.clone() else {
        return;
    };

    if let Some(value) = ctx.bus.get(&signal_name) {
        revealer.set_reveal_child(parse_reveal_value(&value));
    }

    let weak = revealer.downgrade();
    watch_signal(ctx.bus, signal_name, move |value| {
        if let Some(revealer) = weak.upgrade() {
            revealer.set_reveal_child(parse_reveal_value(value));
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });
}

fn parse_transition(value: Option<&str>) -> RevealerTransitionType {
    match value.unwrap_or("slide-down") {
        "none" => RevealerTransitionType::None,
        "crossfade" => RevealerTransitionType::Crossfade,
        "slide-right" => RevealerTransitionType::SlideRight,
        "slide-left" => RevealerTransitionType::SlideLeft,
        "slide-up" => RevealerTransitionType::SlideUp,
        _ => RevealerTransitionType::SlideDown,
    }
}

fn parse_reveal_value(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" | "0" | "false" | "off" | "no" | "hidden" => false,
        "1" | "true" | "on" | "yes" | "visible" | "show" => true,
        _ => true,
    }
}
