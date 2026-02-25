use gtk4::prelude::*;
use gtk4::{Box as GBox, Orientation, Widget};

use crate::config::{WidgetConfig, WidgetProps};
use crate::ui::widgets::build_widget;
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::finalize_widget;

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::VBox(props) = &cfg.props else {
        unreachable!("vbox builder received non-vbox props");
    };

    let container = GBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(props.spacing.unwrap_or(6))
        .build();

    for child in &cfg.base.children {
        container.append(&build_widget(
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
