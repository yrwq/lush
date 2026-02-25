use gtk4::prelude::*;
use gtk4::{PolicyType, ScrolledWindow, Widget};

use crate::config::{WidgetConfig, WidgetProps};
use crate::ui::widgets::build_widget;
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::finalize_widget;

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Scroll(props) = &cfg.props else {
        unreachable!("scroll builder received non-scroll props");
    };

    let container = ScrolledWindow::builder().build();
    container.set_has_frame(false);
    container.set_hscrollbar_policy(parse_policy(
        props.h_policy.as_deref(),
        PolicyType::Automatic,
    ));
    container.set_vscrollbar_policy(parse_policy(
        props.v_policy.as_deref(),
        PolicyType::Automatic,
    ));
    container.set_overlay_scrolling(props.overlay_scrolling.unwrap_or(false));
    if let Some(enabled) = props.kinetic_scrolling {
        container.set_kinetic_scrolling(enabled);
    }
    if let Some(enabled) = props.propagate_natural_width {
        container.set_propagate_natural_width(enabled);
    }
    if let Some(enabled) = props.propagate_natural_height {
        container.set_propagate_natural_height(enabled);
    }
    if let Some(width) = props.min_content_width {
        container.set_min_content_width(width);
    }
    if let Some(height) = props.min_content_height {
        container.set_min_content_height(height);
    }

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
            "scroll uses only first child as content; got {}",
            cfg.base.children.len()
        );
    }

    let widget: Widget = container.upcast();
    finalize_widget(&widget, cfg, ctx.bus, false);
    widget
}

fn parse_policy(raw: Option<&str>, default: PolicyType) -> PolicyType {
    let Some(value) = raw else {
        return default;
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "automatic" | "auto" => PolicyType::Automatic,
        "always" => PolicyType::Always,
        "never" => PolicyType::Never,
        "external" => PolicyType::External,
        other => {
            log::warn!(
                "scroll policy '{}' is invalid; expected automatic|always|never|external; using {:?}",
                other,
                default
            );
            default
        }
    }
}
