use mlua::{Function, Lua, Result, Table};
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use super::data::{
    self, CpuTotalsState, NetworkTotalsState, Provider, ProviderStartContext, ProviderStartOptions,
};
use super::notifications;
use super::signal_bus::SignalBus;

const CALLBACKS_GLOBAL: &str = "__callbacks";
const LUSH_DISPATCH_FN: &str = "__lush_dispatch";
const SCHEDULER_SHUTDOWN_FN: &str = "_lush_scheduler_shutdown";

#[derive(Clone, Debug)]
pub enum AppCommand {
    Open(String),
    Close(String),
    Toggle(String),
    SetVisible(String, bool),
}

type AppCommandListener = Rc<RefCell<Box<dyn FnMut(&AppCommand) -> bool>>>;

#[derive(Default)]
struct DataProviderState {
    refs: u32,
    interval_seconds: u32,
    network_iface: Option<String>,
    disk_path: Option<String>,
    compositor_output: Option<String>,
    handle: Option<data::ProviderHandle>,
}

#[derive(Clone, Debug, Default)]
struct OsdBindingState {
    signals: Vec<String>,
    timeout_ms: u64,
    active: bool,
}

#[derive(Clone, Default)]
pub struct LuaStateBridge {
    bus: Rc<RefCell<Option<SignalBus>>>,
    bootstrap: Rc<RefCell<HashMap<String, String>>>,
    app_command_listeners: Rc<RefCell<HashMap<u64, AppCommandListener>>>,
    next_command_listener_id: Rc<Cell<u64>>,
    pending_app_commands: Rc<RefCell<VecDeque<AppCommand>>>,
    dispatching_app_commands: Rc<Cell<bool>>,
    window_visibility: Rc<RefCell<HashMap<String, bool>>>,
    osd_bindings: Rc<RefCell<HashMap<String, OsdBindingState>>>,
    osd_hide_timers: Rc<RefCell<HashMap<String, glib::SourceId>>>,
    data_providers: Rc<RefCell<HashMap<Provider, DataProviderState>>>,
    notifications_requested: Rc<Cell<bool>>,
    cpu_prev_totals: CpuTotalsState,
    network_prev_totals: NetworkTotalsState,
}

impl LuaStateBridge {
    pub fn attach_signal_bus(&self, bus: SignalBus) {
        for (name, value) in self.bootstrap.borrow().iter() {
            bus.set(name, value);
        }
        *self.bus.borrow_mut() = Some(bus);
        self.start_active_data_providers();
        self.start_requested_notifications_runtime();
        self.start_active_osd_bindings();
    }

    pub fn set(&self, name: &str, value: &str) {
        self.bootstrap
            .borrow_mut()
            .insert(name.to_string(), value.to_string());
        if let Some(bus) = self.bus.borrow().clone() {
            bus.set(name, value);
        }
    }

    pub fn get(&self, name: &str) -> Option<String> {
        if let Some(bus) = self.bus.borrow().clone() {
            return bus.get(name);
        }
        self.bootstrap.borrow().get(name).cloned()
    }

    pub fn snapshot(&self) -> HashMap<String, String> {
        if let Some(bus) = self.bus.borrow().clone() {
            return bus.snapshot();
        }
        self.bootstrap.borrow().clone()
    }

    pub fn with_bus<R>(&self, f: impl FnOnce(&SignalBus) -> R) -> Option<R> {
        self.bus.borrow().as_ref().map(f)
    }

    pub fn data_use(
        &self,
        name: &str,
        interval: Option<u32>,
        iface: Option<String>,
        path: Option<String>,
        output: Option<String>,
    ) -> std::result::Result<(), String> {
        let provider =
            Provider::parse(name).ok_or_else(|| format!("unknown data provider '{}'", name))?;

        let mut providers = self.data_providers.borrow_mut();
        let state = providers.entry(provider).or_default();
        state.refs = state.refs.saturating_add(1);
        if state.refs == 1 {
            state.interval_seconds = interval.unwrap_or(provider.default_interval()).max(1);
            if provider == Provider::Network {
                state.network_iface = iface;
            }
            if provider == Provider::Disk {
                state.disk_path = path.or_else(|| Some("/".to_string()));
            }
            if provider == Provider::Compositor {
                state.compositor_output = output;
            }
            if let Some(bus) = self.bus.borrow().clone() {
                state.handle = Some(data::start_provider(
                    provider,
                    ProviderStartOptions {
                        interval_seconds: state.interval_seconds,
                        network_iface: state.network_iface.clone(),
                        disk_path: state.disk_path.clone(),
                        compositor_output: state.compositor_output.clone(),
                    },
                    ProviderStartContext {
                        bus,
                        cpu_prev_totals: self.cpu_prev_totals.clone(),
                        network_prev_totals: self.network_prev_totals.clone(),
                    },
                )?);
            }
        } else if provider == Provider::Network
            && let Some(iface) = iface
            && state.network_iface.as_deref() != Some(iface.as_str())
        {
            log::warn!(
                "network provider already active with iface={:?}; ignoring new iface={}",
                state.network_iface,
                iface
            );
        } else if provider == Provider::Disk
            && let Some(path) = path
            && state.disk_path.as_deref() != Some(path.as_str())
        {
            log::warn!(
                "disk provider already active with path={:?}; ignoring new path={}",
                state.disk_path,
                path
            );
        } else if provider == Provider::Compositor
            && let Some(output) = output
            && state.compositor_output.as_deref() != Some(output.as_str())
        {
            log::warn!(
                "compositor provider already active with output={:?}; ignoring new output={}",
                state.compositor_output,
                output
            );
        }
        Ok(())
    }

    pub fn data_unuse(&self, name: &str) -> std::result::Result<(), String> {
        let provider =
            Provider::parse(name).ok_or_else(|| format!("unknown data provider '{}'", name))?;

        let mut providers = self.data_providers.borrow_mut();
        let Some(state) = providers.get_mut(&provider) else {
            return Ok(());
        };
        if state.refs == 0 {
            return Ok(());
        }
        state.refs -= 1;
        if state.refs == 0
            && let Some(handle) = state.handle.take()
        {
            data::stop_provider(
                provider,
                handle,
                &self.cpu_prev_totals,
                &self.network_prev_totals,
            );
            state.network_iface = None;
            state.disk_path = None;
            state.compositor_output = None;
        }
        Ok(())
    }

    fn start_active_data_providers(&self) {
        let Some(bus) = self.bus.borrow().clone() else {
            return;
        };

        let mut providers = self.data_providers.borrow_mut();
        for (provider, state) in providers.iter_mut() {
            if state.refs == 0 || state.handle.is_some() {
                continue;
            }
            let interval = state
                .interval_seconds
                .max(1)
                .max(provider.default_interval());
            match data::start_provider(
                *provider,
                ProviderStartOptions {
                    interval_seconds: interval,
                    network_iface: state.network_iface.clone(),
                    disk_path: state.disk_path.clone(),
                    compositor_output: state.compositor_output.clone(),
                },
                ProviderStartContext {
                    bus: bus.clone(),
                    cpu_prev_totals: self.cpu_prev_totals.clone(),
                    network_prev_totals: self.network_prev_totals.clone(),
                },
            ) {
                Ok(handle) => {
                    state.handle = Some(handle);
                    state.interval_seconds = interval;
                }
                Err(err) => {
                    log::warn!(
                        "failed to start data provider '{}': {}",
                        provider.as_str(),
                        err
                    );
                }
            }
        }
    }

    pub fn request_notifications_runtime(&self) {
        self.notifications_requested.set(true);
        self.start_requested_notifications_runtime();
    }

    fn start_requested_notifications_runtime(&self) {
        if !self.notifications_requested.get() {
            return;
        }
        let Some(bus) = self.bus.borrow().clone() else {
            return;
        };
        notifications::start(bus);
    }

    pub fn osd_bind(
        &self,
        name: &str,
        signals: Vec<String>,
        timeout_ms: u64,
    ) -> std::result::Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("osd name cannot be empty".to_string());
        }
        let signals: Vec<String> = signals
            .into_iter()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect();
        if signals.is_empty() {
            return Err("osd signals cannot be empty".to_string());
        }

        let timeout_ms = timeout_ms.max(1);
        let mut bindings = self.osd_bindings.borrow_mut();
        let state = bindings.entry(name.to_string()).or_default();
        state.signals = signals;
        state.timeout_ms = timeout_ms;
        state.active = false;
        drop(bindings);

        self.start_active_osd_bindings();
        Ok(())
    }

    fn start_active_osd_bindings(&self) {
        let Some(bus) = self.bus.borrow().clone() else {
            return;
        };
        let mut bindings = self.osd_bindings.borrow_mut();
        for (name, state) in bindings.iter_mut() {
            if state.active || state.signals.is_empty() {
                continue;
            }
            state.active = true;
            let osd_name = name.clone();
            for signal_name in state.signals.clone() {
                let bridge = self.clone();
                let osd_name = osd_name.clone();
                bus.subscribe_key(&signal_name, move |_| {
                    bridge.trigger_osd(&osd_name);
                    true
                });
            }
        }
    }

    fn trigger_osd(&self, name: &str) {
        let timeout_ms = self
            .osd_bindings
            .borrow()
            .get(name)
            .map(|v| v.timeout_ms)
            .unwrap_or(1200)
            .max(1);
        self.queue_app_command(AppCommand::SetVisible(name.to_string(), true));

        if let Some(source) = self.osd_hide_timers.borrow_mut().remove(name) {
            source.remove();
        }

        let bridge = self.clone();
        let name = name.to_string();
        let timer_key = name.clone();
        let source = glib::timeout_add_local(Duration::from_millis(timeout_ms), move || {
            bridge.queue_app_command(AppCommand::SetVisible(name.clone(), false));
            bridge.osd_hide_timers.borrow_mut().remove(&name);
            glib::ControlFlow::Break
        });
        self.osd_hide_timers.borrow_mut().insert(timer_key, source);
    }

    fn shutdown(&self) {
        for (_, source) in self.osd_hide_timers.borrow_mut().drain() {
            source.remove();
        }
        for state in self.osd_bindings.borrow_mut().values_mut() {
            state.active = false;
        }

        let mut providers = self.data_providers.borrow_mut();
        for (provider, state) in providers.iter_mut() {
            if let Some(handle) = state.handle.take() {
                data::stop_provider(
                    *provider,
                    handle,
                    &self.cpu_prev_totals,
                    &self.network_prev_totals,
                );
            }
            state.refs = 0;
            state.network_iface = None;
            state.disk_path = None;
            state.compositor_output = None;
        }

        *self.bus.borrow_mut() = None;
    }

    pub fn queue_app_command(&self, cmd: AppCommand) {
        self.pending_app_commands.borrow_mut().push_back(cmd);
        if self.dispatching_app_commands.get() {
            return;
        }

        self.dispatching_app_commands.set(true);
        loop {
            let cmd = {
                let mut pending = self.pending_app_commands.borrow_mut();
                pending.pop_front()
            };
            let Some(cmd) = cmd else {
                break;
            };
            let listeners: Vec<(u64, AppCommandListener)> = self
                .app_command_listeners
                .borrow()
                .iter()
                .map(|(id, cb)| (*id, cb.clone()))
                .collect();
            let mut stale = Vec::new();
            for (id, callback) in listeners {
                if !(callback.borrow_mut())(&cmd) {
                    stale.push(id);
                }
            }
            if !stale.is_empty() {
                let mut refs = self.app_command_listeners.borrow_mut();
                for id in stale {
                    refs.remove(&id);
                }
            }
        }
        self.dispatching_app_commands.set(false);
    }

    pub fn subscribe_app_commands<F>(&self, callback: F) -> u64
    where
        F: FnMut(&AppCommand) -> bool + 'static,
    {
        let id = self.next_command_listener_id.get().saturating_add(1);
        self.next_command_listener_id.set(id);
        self.app_command_listeners
            .borrow_mut()
            .insert(id, Rc::new(RefCell::new(Box::new(callback))));
        id
    }

    pub fn set_window_visible(&self, name: &str, visible: bool) {
        self.window_visibility
            .borrow_mut()
            .insert(name.to_string(), visible);
    }

    pub fn is_window_visible(&self, name: &str) -> bool {
        self.window_visibility
            .borrow()
            .get(name)
            .copied()
            .unwrap_or(false)
    }

    pub fn window_names(&self) -> Vec<String> {
        self.window_visibility.borrow().keys().cloned().collect()
    }
}

pub struct LuaRuntime {
    lua: Lua,
    bridge: LuaStateBridge,
}

impl LuaRuntime {
    pub fn new(lua: Lua, bridge: LuaStateBridge) -> Self {
        Self { lua, bridge }
    }

    pub fn attach_signal_bus(&self, bus: SignalBus) {
        self.bridge.attach_signal_bus(bus);
    }

    pub fn subscribe_app_commands<F>(&self, callback: F) -> u64
    where
        F: FnMut(&AppCommand) -> bool + 'static,
    {
        self.bridge.subscribe_app_commands(callback)
    }

    pub fn queue_app_command(&self, cmd: AppCommand) {
        self.bridge.queue_app_command(cmd);
    }

    pub fn set_window_visible(&self, name: &str, visible: bool) {
        self.bridge.set_window_visible(name, visible);
    }

    pub fn is_window_visible(&self, name: &str) -> bool {
        self.bridge.is_window_visible(name)
    }

    pub fn window_names(&self) -> Vec<String> {
        self.bridge.window_names()
    }

    pub fn data_use(
        &self,
        name: &str,
        interval: Option<u32>,
        iface: Option<String>,
        path: Option<String>,
        output: Option<String>,
    ) -> std::result::Result<(), String> {
        self.bridge.data_use(name, interval, iface, path, output)
    }

    pub fn invoke_click_callback(&self, id: u64) -> Result<()> {
        let callbacks: Table = self.lua.globals().get(CALLBACKS_GLOBAL)?;
        let callback: Function = callbacks.get(id)?;
        callback.call::<()>(())
    }

    pub fn dispatch_signal(&self, name: &str, value: &str) -> Result<()> {
        let globals = self.lua.globals();
        let dispatch: Function = globals.get(LUSH_DISPATCH_FN)?;
        dispatch.call::<()>((name, value))
    }
}

impl Drop for LuaRuntime {
    fn drop(&mut self) {
        if let Ok(shutdown) = self.lua.globals().get::<Function>(SCHEDULER_SHUTDOWN_FN) {
            let _ = shutdown.call::<()>(());
        }
        self.bridge.shutdown();
    }
}
