use gtk4::prelude::*;
use gtk4::{MenuButton, Popover, PositionType, Widget};

use crate::config::{PopoverProps, WidgetConfig, WidgetProps};
use crate::ui::widgets::build_widget;
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::finalize_widget;

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Popover(props) = &cfg.props else {
        unreachable!("popover builder received non-popover props");
    };

    let button = MenuButton::new();
    button.set_has_frame(false);
    let popover = Popover::new();
    popover.add_css_class("quick-popover-surface");
    configure_popover(&popover, props);

    if let Some(trigger_cfg) = cfg.base.children.first() {
        let trigger = build_widget(
            trigger_cfg,
            ctx.bus,
            ctx.windows,
            ctx.loaded,
            ctx.default_output,
        );
        button.set_child(Some(&trigger));
    } else {
        let fallback = gtk4::Label::new(Some("menu"));
        button.set_child(Some(&fallback));
    }

    if let Some(content_cfg) = cfg.base.children.get(1) {
        let content = build_widget(
            content_cfg,
            ctx.bus,
            ctx.windows,
            ctx.loaded,
            ctx.default_output,
        );
        popover.set_child(Some(&content));
    }

    if cfg.base.children.len() > 2 {
        log::warn!(
            "popover uses only first 2 children; got {}",
            cfg.base.children.len()
        );
    }

    button.set_popover(Some(&popover));

    let widget: Widget = button.upcast();
    finalize_widget(&widget, cfg, ctx.bus, true);
    widget
}

fn configure_popover(popover: &Popover, props: &PopoverProps) {
    popover.set_autohide(props.autohide.unwrap_or(true));
    popover.set_has_arrow(props.has_arrow.unwrap_or(true));

    if let Some(position) = props.position.as_deref() {
        popover.set_position(parse_position(position));
    }
}

fn parse_position(raw: &str) -> PositionType {
    match raw.trim().to_ascii_lowercase().as_str() {
        "top" => PositionType::Top,
        "left" => PositionType::Left,
        "right" => PositionType::Right,
        _ => PositionType::Bottom,
    }
}
