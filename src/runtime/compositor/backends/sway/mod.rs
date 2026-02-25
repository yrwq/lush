mod ipc;
mod toplevels;
mod tree;
mod workspaces;

use async_channel::Receiver;

use crate::runtime::compositor::{CompositorSnapshot, ToplevelEntry};

pub fn available() -> bool {
    let Some(path) = ipc::socket_path() else {
        return false;
    };
    std::os::unix::net::UnixStream::connect(path).is_ok()
}

pub fn focus_workspace(index_1_based: u32) -> Result<(), String> {
    ipc::run_command(&format!("workspace number {}", index_1_based))
}

pub fn subscribe_snapshots(output_selector: Option<&str>) -> Option<Receiver<CompositorSnapshot>> {
    workspaces::subscribe_snapshots(output_selector)
}

pub fn subscribe_toplevels(output_selector: Option<&str>) -> Option<Receiver<Vec<ToplevelEntry>>> {
    toplevels::subscribe(output_selector)
}
