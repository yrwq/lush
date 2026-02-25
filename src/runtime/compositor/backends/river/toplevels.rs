use async_channel::{Receiver, Sender};
use std::collections::{HashMap, HashSet};
use std::thread;
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_output, wl_registry};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use crate::protocols::wlr_foreign_toplevel_management::wlr_foreign_toplevel_management_unstable_v1::{
    zwlr_foreign_toplevel_handle_v1, zwlr_foreign_toplevel_manager_v1,
};

use crate::runtime::compositor::ToplevelEntry;

pub(super) fn subscribe(output_selector: Option<&str>) -> Option<Receiver<Vec<ToplevelEntry>>> {
    if !crate::runtime::river::river_status_available() {
        return None;
    }

    let selector = output_selector.map(str::to_string);
    let (tx, rx) = async_channel::unbounded();
    thread::Builder::new()
        .name("river-toplevels-listener".to_string())
        .spawn(move || run_toplevel_listener(selector, tx))
        .ok()?;
    Some(rx)
}

#[derive(Default)]
struct RiverToplevelInfo {
    title: String,
    app_id: String,
    identifier: String,
    outputs: HashSet<wayland_client::backend::ObjectId>,
    focused: bool,
    pending_focused: Option<bool>,
    focused_epoch: u64,
}

struct RiverToplevelState {
    selector: Option<String>,
    tx: Sender<Vec<ToplevelEntry>>,
    output_order: Vec<wayland_client::backend::ObjectId>,
    output_name_by_id: HashMap<wayland_client::backend::ObjectId, String>,
    output_global_name_by_id: HashMap<wayland_client::backend::ObjectId, u32>,
    output_by_global_name: HashMap<u32, wl_output::WlOutput>,
    handles: HashMap<wayland_client::backend::ObjectId, RiverToplevelInfo>,
    handle_order: Vec<wayland_client::backend::ObjectId>,
    last_sent: Vec<ToplevelEntry>,
    focus_epoch_seq: u64,
}

impl RiverToplevelState {
    fn new(selector: Option<String>, tx: Sender<Vec<ToplevelEntry>>) -> Self {
        Self {
            selector,
            tx,
            output_order: Vec::new(),
            output_name_by_id: HashMap::new(),
            output_global_name_by_id: HashMap::new(),
            output_by_global_name: HashMap::new(),
            handles: HashMap::new(),
            handle_order: Vec::new(),
            last_sent: Vec::new(),
            focus_epoch_seq: 0,
        }
    }

    fn bind_output(
        &mut self,
        registry: &wl_registry::WlRegistry,
        name: u32,
        version: u32,
        qh: &QueueHandle<Self>,
    ) {
        let output: wl_output::WlOutput = registry.bind(name, version.min(4), qh, name);
        if !self.output_order.contains(&output.id()) {
            self.output_order.push(output.id());
        }
        self.output_global_name_by_id.insert(output.id(), name);
        self.output_by_global_name.insert(name, output);
    }

    fn emit_if_changed(&mut self) {
        let next = self.filtered_snapshot();
        if next != self.last_sent {
            self.last_sent = next.clone();
            let _ = self.tx.try_send(next);
        }
    }

    fn filtered_snapshot(&self) -> Vec<ToplevelEntry> {
        let selected_output = self.resolve_selected_output_id();
        let mut out = Vec::new();
        let mut out_focus_epochs = Vec::new();
        let mut focused_count = 0_usize;
        for handle_id in &self.handle_order {
            let Some(info) = self.handles.get(handle_id) else {
                continue;
            };
            if let Some(output_id) = selected_output.as_ref()
                && !info.outputs.contains(output_id)
            {
                continue;
            }

            let title = info.title.trim().to_string();
            let app_id = info.app_id.trim().to_string();
            if title.is_empty() && app_id.is_empty() {
                continue;
            }
            out.push(ToplevelEntry {
                title,
                app_id,
                identifier: info.identifier.trim().to_string(),
                workspace_id: None,
                workspace_mask: 0,
                focused: info.focused,
            });
            out_focus_epochs.push(if info.focused { info.focused_epoch } else { 0 });
            if info.focused {
                focused_count += 1;
            }
        }
        if focused_count > 1 {
            let keep_idx = out_focus_epochs
                .iter()
                .enumerate()
                .max_by_key(|(_, epoch)| **epoch)
                .map(|(idx, _)| idx);
            if let Some(keep_idx) = keep_idx {
                for (idx, entry) in out.iter_mut().enumerate() {
                    if idx != keep_idx {
                        entry.focused = false;
                    }
                }
            }
        }
        out
    }

    fn resolve_selected_output_id(&self) -> Option<wayland_client::backend::ObjectId> {
        let selector = self.selector.as_deref().unwrap_or("focused").trim();
        if selector.is_empty() || selector.eq_ignore_ascii_case("focused") {
            let focused = crate::runtime::river::focused_output_name()?;
            return self
                .output_name_by_id
                .iter()
                .find_map(|(id, name)| name.eq_ignore_ascii_case(&focused).then_some(id.clone()));
        }

        if let Ok(index) = selector.parse::<usize>() {
            return self.output_order.get(index).cloned();
        }
        if let Ok(global) = selector.parse::<u32>() {
            return self
                .output_global_name_by_id
                .iter()
                .find_map(|(id, g)| (*g == global).then_some(id.clone()));
        }

        let target = selector.to_ascii_lowercase();
        self.output_name_by_id
            .iter()
            .find_map(|(id, name)| (name.to_ascii_lowercase() == target).then_some(id.clone()))
            .or_else(|| {
                self.output_name_by_id.iter().find_map(|(id, name)| {
                    name.to_ascii_lowercase()
                        .contains(&target)
                        .then_some(id.clone())
                })
            })
    }
}

fn run_toplevel_listener(selector: Option<String>, tx: Sender<Vec<ToplevelEntry>>) {
    let conn = match Connection::connect_to_env() {
        Ok(conn) => conn,
        Err(_) => return,
    };
    let (globals, mut event_queue) = match registry_queue_init::<RiverToplevelState>(&conn) {
        Ok(v) => v,
        Err(_) => return,
    };
    let qh = event_queue.handle();
    let mut state = RiverToplevelState::new(selector, tx);

    let _manager: zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1 =
        match globals.bind(&qh, 1..=3, ()) {
            Ok(v) => v,
            Err(_) => return,
        };

    let snapshot = globals.contents().clone_list();
    let registry = globals.registry();
    for global in &snapshot {
        if global.interface == wl_output::WlOutput::interface().name {
            state.bind_output(registry, global.name, global.version, &qh);
        }
    }
    state.emit_if_changed();

    loop {
        if event_queue.blocking_dispatch(&mut state).is_err() {
            break;
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for RiverToplevelState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == wl_output::WlOutput::interface().name {
                    state.bind_output(registry, name, version, qh);
                    state.emit_if_changed();
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(output) = state.output_by_global_name.remove(&name) {
                    let id = output.id();
                    state.output_order.retain(|entry| entry != &id);
                    state.output_name_by_id.remove(&id);
                    state.output_global_name_by_id.remove(&id);
                    for info in state.handles.values_mut() {
                        info.outputs.remove(&id);
                    }
                    state.emit_if_changed();
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, u32> for RiverToplevelState {
    fn event(
        state: &mut Self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name } = event {
            state.output_name_by_id.insert(output.id(), name);
            state.emit_if_changed();
        }
    }
}

impl Dispatch<zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1, ()>
    for RiverToplevelState
{
    fn event(
        state: &mut Self,
        _: &zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } => {
                let id = toplevel.id();
                if !state.handles.contains_key(&id) {
                    state.handle_order.push(id.clone());
                }
                state.handles.entry(id).or_default();
                state.emit_if_changed();
            }
            zwlr_foreign_toplevel_manager_v1::Event::Finished => {}
        }
    }

    wayland_client::event_created_child!(
        RiverToplevelState,
        zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
        [
            zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (
                zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
                ()
            )
        ]
    );
}

impl Dispatch<zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1, ()>
    for RiverToplevelState
{
    fn event(
        state: &mut Self,
        handle: &zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let info = state.handles.entry(handle.id()).or_default();
        let mut changed = false;
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                if info.title != title {
                    info.title = title;
                    changed = true;
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                if info.app_id != app_id {
                    info.app_id = app_id;
                    changed = true;
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputEnter { output } => {
                if info.outputs.insert(output.id()) {
                    changed = true;
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputLeave { output } => {
                if info.outputs.remove(&output.id()) {
                    changed = true;
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::State { state } => {
                let focused = state_contains_activated(&state);
                if info.pending_focused != Some(focused) {
                    info.pending_focused = Some(focused);
                    changed = true;
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                let id = handle.id();
                state.handles.remove(&id);
                state.handle_order.retain(|entry| entry != &id);
                changed = true;
            }
            zwlr_foreign_toplevel_handle_v1::Event::Done => {
                if let Some(next_focused) = info.pending_focused.take()
                    && info.focused != next_focused
                {
                    info.focused = next_focused;
                    if next_focused {
                        state.focus_epoch_seq = state.focus_epoch_seq.saturating_add(1);
                        info.focused_epoch = state.focus_epoch_seq;
                    }
                    changed = true;
                }
            }
            _ => {}
        }
        if changed {
            state.emit_if_changed();
        }
    }
}

fn state_contains_activated(state: &[u8]) -> bool {
    const ACTIVATED: u32 = 2;
    state
        .chunks_exact(4)
        .map(|chunk| u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .any(|value| value == ACTIVATED)
}
