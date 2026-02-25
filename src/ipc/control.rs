use std::sync::mpsc;
use std::time::Duration;

use zbus::blocking::connection::Builder as ConnectionBuilder;
use zbus::interface;

pub const BUS_NAME: &str = "xyz.yrwq.lush.control";
pub const BUS_PATH: &str = "/xyz/yrwq/lush";
pub const BUS_INTERFACE: &str = "xyz.yrwq.lush.Control";

pub enum ControlRequest {
    OpenWindow(String),
    CloseWindow(String),
    ToggleWindow(String),
    SetWindowVisible(String, bool),
    ListWindows(mpsc::Sender<Vec<String>>),
    ReloadConfig(mpsc::Sender<Result<(), String>>),
    ReloadCss(mpsc::Sender<Result<(), String>>),
    Ping(mpsc::Sender<String>),
}

struct ControlIface {
    tx: async_channel::Sender<ControlRequest>,
}

impl ControlIface {
    fn new(tx: async_channel::Sender<ControlRequest>) -> Self {
        Self { tx }
    }
}

#[interface(name = "xyz.yrwq.lush.Control")]
impl ControlIface {
    #[zbus(name = "OpenWindow")]
    fn open_window(&self, name: String) -> zbus::fdo::Result<()> {
        self.tx
            .send_blocking(ControlRequest::OpenWindow(name))
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    #[zbus(name = "CloseWindow")]
    fn close_window(&self, name: String) -> zbus::fdo::Result<()> {
        self.tx
            .send_blocking(ControlRequest::CloseWindow(name))
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    #[zbus(name = "ToggleWindow")]
    fn toggle_window(&self, name: String) -> zbus::fdo::Result<()> {
        self.tx
            .send_blocking(ControlRequest::ToggleWindow(name))
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    #[zbus(name = "SetWindowVisible")]
    fn set_window_visible(&self, name: String, visible: bool) -> zbus::fdo::Result<()> {
        self.tx
            .send_blocking(ControlRequest::SetWindowVisible(name, visible))
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    #[zbus(name = "ListWindows")]
    fn list_windows(&self) -> zbus::fdo::Result<Vec<String>> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send_blocking(ControlRequest::ListWindows(reply_tx))
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;
        reply_rx
            .recv()
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))
    }

    #[zbus(name = "ReloadCss")]
    fn reload_css(&self) -> zbus::fdo::Result<()> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send_blocking(ControlRequest::ReloadCss(reply_tx))
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;

        match reply_rx.recv() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(err)) => Err(zbus::fdo::Error::Failed(err)),
            Err(err) => Err(zbus::fdo::Error::Failed(err.to_string())),
        }
    }

    #[zbus(name = "Reload")]
    fn reload(&self) -> zbus::fdo::Result<()> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send_blocking(ControlRequest::ReloadConfig(reply_tx))
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;

        match reply_rx.recv() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(err)) => Err(zbus::fdo::Error::Failed(err)),
            Err(err) => Err(zbus::fdo::Error::Failed(err.to_string())),
        }
    }

    #[zbus(name = "Ping")]
    fn ping(&self) -> zbus::fdo::Result<String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send_blocking(ControlRequest::Ping(reply_tx))
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;
        reply_rx
            .recv()
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))
    }
}

pub fn start_service(tx: async_channel::Sender<ControlRequest>) {
    std::thread::spawn(move || {
        if let Err(err) = run_server(tx) {
            log::warn!("control ipc: failed to start service: {}", err);
        }
    });
}

fn run_server(tx: async_channel::Sender<ControlRequest>) -> zbus::Result<()> {
    let _connection = ConnectionBuilder::session()?
        .name(BUS_NAME)?
        .serve_at(BUS_PATH, ControlIface::new(tx))?
        .build()?;

    log::info!("control ipc: serving {} ({})", BUS_NAME, BUS_INTERFACE);
    loop {
        std::thread::park_timeout(Duration::from_secs(3600));
    }
}
