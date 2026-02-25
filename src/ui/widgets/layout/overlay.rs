use gtk4::prelude::*;
use gtk4::{Overlay as GOverlay, Widget};

use crate::config::{WidgetConfig, WidgetProps};
use crate::ui::widgets::build_widget;
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::finalize_widget;

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Overlay(_) = &cfg.props else {
        unreachable!("overlay builder received non-overlay props");
    };

    let container = GOverlay::new();

    if let Some(base_child) = cfg.base.children.first() {
        container.set_child(Some(&build_widget(
            base_child,
            ctx.bus,
            ctx.windows,
            ctx.loaded,
            ctx.default_output,
        )));
    }

    for child in cfg.base.children.iter().skip(1) {
        container.add_overlay(&build_widget(
            child,
            ctx.bus,
            ctx.windows,
            ctx.loaded,
            ctx.default_output,
        ));
    }

    let widget: Widget = container.upcast();
    finalize_widget(&widget, cfg, ctx.bus, false);
    widget
}
