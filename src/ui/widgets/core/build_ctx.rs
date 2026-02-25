use crate::config::LoadedConfig;
use crate::runtime::signal_bus::SignalBus;
use crate::ui::app::WindowRegistry;

pub struct WidgetBuildCtx<'a> {
    pub bus: &'a SignalBus,
    pub windows: &'a WindowRegistry,
    pub loaded: &'a LoadedConfig,
    pub default_output: Option<&'a str>,
}
