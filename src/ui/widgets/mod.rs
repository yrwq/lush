mod content;
mod core;
mod integrations;
mod layout;

use gtk4::Widget;

use crate::config::{LoadedConfig, WidgetConfig, WidgetKind};
use crate::runtime::signal_bus::SignalBus;
use crate::ui::app::WindowRegistry;

use core::build_ctx::WidgetBuildCtx;

pub fn build_widget(
    cfg: &WidgetConfig,
    bus: &SignalBus,
    windows: &WindowRegistry,
    loaded: &LoadedConfig,
    default_output: Option<&str>,
) -> Widget {
    let ctx = WidgetBuildCtx {
        bus,
        windows,
        loaded,
        default_output,
    };

    match cfg.kind {
        WidgetKind::HBox => layout::hbox::build(cfg, &ctx),
        WidgetKind::VBox => layout::vbox::build(cfg, &ctx),
        WidgetKind::CenterBox => layout::centerbox::build(cfg, &ctx),
        WidgetKind::Revealer => layout::revealer::build(cfg, &ctx),
        WidgetKind::Scroll => layout::scroll::build(cfg, &ctx),
        WidgetKind::Overlay => layout::overlay::build(cfg, &ctx),
        WidgetKind::List => layout::list::build(cfg, &ctx),
        WidgetKind::Popover => content::popover::build(cfg, &ctx),
        WidgetKind::Label => content::label::build(cfg, &ctx),
        WidgetKind::Button => content::button::build(cfg, &ctx),
        WidgetKind::Clock => content::clock::build(cfg, &ctx),
        WidgetKind::Workspaces => integrations::workspaces::build(cfg, &ctx),
        WidgetKind::Dock => integrations::dock::build(cfg, &ctx),
        WidgetKind::Image => content::image::build(cfg, &ctx),
        WidgetKind::Progress => content::progress::build(cfg, &ctx),
        WidgetKind::Slider => content::slider::build(cfg, &ctx),
    }
}
