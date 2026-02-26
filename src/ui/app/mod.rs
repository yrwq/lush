use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk4::ApplicationWindow;

use crate::runtime::lua_runtime::LuaRuntime;
use crate::runtime::signal_bus::SignalBus;
use crate::ui::style::StyleManager;

mod build;
mod control;

pub use build::{build_windows, reconfigure_windows};
pub type WindowRegistry = Rc<RefCell<HashMap<String, ApplicationWindow>>>;

pub struct UiSession {
    pub runtime: Rc<LuaRuntime>,
    pub bus: SignalBus,
    pub windows: WindowRegistry,
    pub style: StyleManager,
}
