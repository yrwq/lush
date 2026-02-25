use gtk4::prelude::*;
use gtk4::{CenterBox as GCenterBox, Widget};

use crate::config::{WidgetConfig, WidgetProps};
use crate::ui::widgets::build_widget;
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::finalize_widget;

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::CenterBox(_props) = &cfg.props else {
        unreachable!("centerbox builder received non-centerbox props");
    };

    let container = GCenterBox::new();

    if let Some(start_cfg) = cfg.base.children.first() {
        let start = build_widget(
            start_cfg,
            ctx.bus,
            ctx.windows,
            ctx.loaded,
            ctx.default_output,
        );
        container.set_start_widget(Some(&start));
    }

    if let Some(center_cfg) = cfg.base.children.get(1) {
        let center = build_widget(
            center_cfg,
            ctx.bus,
            ctx.windows,
            ctx.loaded,
            ctx.default_output,
        );
        container.set_center_widget(Some(&center));
    }

    if let Some(end_cfg) = cfg.base.children.get(2) {
        let end = build_widget(
            end_cfg,
            ctx.bus,
            ctx.windows,
            ctx.loaded,
            ctx.default_output,
        );
        container.set_end_widget(Some(&end));
    }

    let widget: Widget = container.upcast();
    finalize_widget(&widget, cfg, ctx.bus, false);
    widget
}
