use gtk4::prelude::*;
use gtk4::{Box as GBox, Orientation, Widget};

use crate::config::{WidgetConfig, WidgetProps};
use crate::ui::widgets::build_widget;
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::finalize_widget;
use crate::ui::widgets::core::template::expand_widget_template;

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::List(props) = &cfg.props else {
        unreachable!("list builder received non-list props");
    };

    let orientation = match props.orientation.as_deref() {
        Some("horizontal") => Orientation::Horizontal,
        _ => Orientation::Vertical,
    };
    let spacing = props.spacing.unwrap_or(0);
    let count = props.count.unwrap_or(1).clamp(1, 256) as usize;
    let base = props.bind.clone().unwrap_or_else(|| "item.".to_string());

    let container = GBox::new(orientation, spacing);
    container.add_css_class("list");

    let Some(template) = cfg.base.children.first() else {
        log::warn!("list requires one template child in 'children'");
        let widget: Widget = container.upcast();
        finalize_widget(&widget, cfg, ctx.bus, false);
        return widget;
    };

    for index in 1..=count {
        let mut row_cfg = template.clone();
        expand_widget_template(&mut row_cfg, &base, index);
        let row_widget = build_widget(
            &row_cfg,
            ctx.bus,
            ctx.windows,
            ctx.loaded,
            ctx.default_output,
        );
        container.append(&row_widget);
    }

    let widget: Widget = container.upcast();
    finalize_widget(&widget, cfg, ctx.bus, false);
    widget
}
