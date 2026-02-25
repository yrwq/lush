use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use glib::SourceId;

use crate::runtime::signal_bus::SignalBus;

mod audio;
mod battery;
mod bluetooth;
mod compositor;
mod cpu;
mod disk;
mod evented;
mod memory;
mod mpris;
mod network;

#[derive(Clone, Copy, Debug)]
pub struct CpuTotals {
    pub total_ticks: u64,
    pub idle_ticks: u64,
}

pub type CpuTotalsState = Rc<RefCell<Option<CpuTotals>>>;

#[derive(Clone, Copy, Debug)]
pub struct NetworkTotals {
    pub down_total_bytes: u64,
    pub up_total_bytes: u64,
    pub collected_at: Instant,
}

pub type NetworkTotalsState = Rc<RefCell<Option<NetworkTotals>>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Provider {
    Cpu,
    Memory,
    Network,
    Disk,
    Battery,
    Audio,
    Bluetooth,
    Mpris,
    Compositor,
}

pub enum ProviderHandle {
    Timer(SourceId),
    NetworkWatcher(network::NetworkWatcher),
    BatteryWatcher(battery::BatteryWatcher),
    DiskWatcher(disk::DiskWatcher),
    AudioWatcher(audio::AudioWatcher),
    BluetoothWatcher(bluetooth::BluetoothWatcher),
    MprisWatcher(mpris::MprisWatcher),
    CompositorWatcher(compositor::CompositorWatcher),
}

#[derive(Clone, Debug, Default)]
pub struct ProviderStartOptions {
    pub interval_seconds: u32,
    pub network_iface: Option<String>,
    pub disk_path: Option<String>,
    pub compositor_output: Option<String>,
}

#[derive(Clone)]
pub struct ProviderStartContext {
    pub bus: SignalBus,
    pub cpu_prev_totals: CpuTotalsState,
    pub network_prev_totals: NetworkTotalsState,
}

impl Provider {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "cpu" => Some(Self::Cpu),
            "memory" => Some(Self::Memory),
            "network" => Some(Self::Network),
            "disk" => Some(Self::Disk),
            "battery" => Some(Self::Battery),
            "audio" => Some(Self::Audio),
            "bluetooth" => Some(Self::Bluetooth),
            "mpris" => Some(Self::Mpris),
            "compositor" => Some(Self::Compositor),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Memory => "memory",
            Self::Network => "network",
            Self::Disk => "disk",
            Self::Battery => "battery",
            Self::Audio => "audio",
            Self::Bluetooth => "bluetooth",
            Self::Mpris => "mpris",
            Self::Compositor => "compositor",
        }
    }

    pub fn default_interval(self) -> u32 {
        match self {
            Self::Cpu => 2,
            Self::Memory => 3,
            Self::Network => 1,
            Self::Disk => 10,
            Self::Battery => 10,
            Self::Audio => 10,
            Self::Bluetooth => 120,
            Self::Mpris => 20,
            Self::Compositor => 1,
        }
    }
}

pub fn start_provider(
    provider: Provider,
    options: ProviderStartOptions,
    ctx: ProviderStartContext,
) -> Result<ProviderHandle, String> {
    let interval = options.interval_seconds.max(1);

    match provider {
        Provider::Cpu => {
            cpu::collect(&ctx.bus, &ctx.cpu_prev_totals)?;
            Ok(ProviderHandle::Timer(glib::timeout_add_seconds_local(
                interval,
                move || {
                    if let Err(err) = cpu::collect(&ctx.bus, &ctx.cpu_prev_totals) {
                        log::debug!("data.cpu collector error: {}", err);
                    }
                    glib::ControlFlow::Continue
                },
            )))
        }
        Provider::Memory => {
            memory::collect(&ctx.bus)?;
            Ok(ProviderHandle::Timer(glib::timeout_add_seconds_local(
                interval,
                move || {
                    if let Err(err) = memory::collect(&ctx.bus) {
                        log::debug!("data.memory collector error: {}", err);
                    }
                    glib::ControlFlow::Continue
                },
            )))
        }
        Provider::Network => {
            let watcher = network::start_evented(
                ctx.bus,
                options.network_iface,
                ctx.network_prev_totals,
                interval,
            )?;
            Ok(ProviderHandle::NetworkWatcher(watcher))
        }
        Provider::Disk => {
            let watcher = disk::start_evented(ctx.bus, options.disk_path, interval)?;
            Ok(ProviderHandle::DiskWatcher(watcher))
        }
        Provider::Battery => {
            let watcher = battery::start_evented(ctx.bus)?;
            Ok(ProviderHandle::BatteryWatcher(watcher))
        }
        Provider::Audio => {
            let watcher = audio::start_evented(ctx.bus)?;
            Ok(ProviderHandle::AudioWatcher(watcher))
        }
        Provider::Bluetooth => Ok(ProviderHandle::BluetoothWatcher(bluetooth::start_evented(
            ctx.bus, interval,
        ))),
        Provider::Mpris => Ok(ProviderHandle::MprisWatcher(mpris::start_evented(
            ctx.bus, interval,
        ))),
        Provider::Compositor => Ok(ProviderHandle::CompositorWatcher(
            compositor::start_evented(ctx.bus, options.compositor_output),
        )),
    }
}

pub fn stop_provider(
    provider: Provider,
    handle: ProviderHandle,
    cpu_prev_totals: &CpuTotalsState,
    network_prev_totals: &NetworkTotalsState,
) {
    match handle {
        ProviderHandle::Timer(source) => {
            source.remove();
        }
        ProviderHandle::NetworkWatcher(_watcher) => {}
        ProviderHandle::BatteryWatcher(_watcher) => {}
        ProviderHandle::DiskWatcher(_watcher) => {}
        ProviderHandle::AudioWatcher(_watcher) => {}
        ProviderHandle::BluetoothWatcher(_watcher) => {}
        ProviderHandle::MprisWatcher(_watcher) => {}
        ProviderHandle::CompositorWatcher(_watcher) => {}
    }
    match provider {
        Provider::Cpu => {
            *cpu_prev_totals.borrow_mut() = None;
        }
        Provider::Network => {
            *network_prev_totals.borrow_mut() = None;
        }
        Provider::Memory
        | Provider::Disk
        | Provider::Battery
        | Provider::Audio
        | Provider::Bluetooth
        | Provider::Mpris
        | Provider::Compositor => {}
    }
}
