use std::collections::HashMap;

use wayland_client::globals::GlobalList;
use wayland_client::protocol::{wl_output, wl_registry, wl_seat};
use wayland_client::{Proxy, QueueHandle};

use crate::protocols::river_status::river_status_unstable_v1::{
    zriver_output_status_v1, zriver_seat_status_v1, zriver_status_manager_v1,
};

use super::model::RiverSnapshot;
use super::model::{LatestSnapshotsStore, OutputState, SnapshotListeners};

pub(super) struct RiverState {
    pub(super) listeners: SnapshotListeners,
    pub(super) latest: LatestSnapshotsStore,
    pub(super) manager: Option<zriver_status_manager_v1::ZriverStatusManagerV1>,
    pub(super) seat: Option<wl_seat::WlSeat>,
    pub(super) seat_status: Option<zriver_seat_status_v1::ZriverSeatStatusV1>,
    pub(super) output_statuses:
        HashMap<wayland_client::backend::ObjectId, zriver_output_status_v1::ZriverOutputStatusV1>,
    pub(super) focused_output: Option<wayland_client::backend::ObjectId>,
    pub(super) last_updated_output: Option<wayland_client::backend::ObjectId>,
    pub(super) output_order: Vec<wayland_client::backend::ObjectId>,
    pub(super) output_global_name_by_id: HashMap<wayland_client::backend::ObjectId, u32>,
    pub(super) output_name_by_id: HashMap<wayland_client::backend::ObjectId, String>,
    pub(super) output_states: HashMap<wayland_client::backend::ObjectId, OutputState>,
    pub(super) output_view_tags: HashMap<wayland_client::backend::ObjectId, Vec<u32>>,
    pub(super) output_by_global_name: HashMap<u32, wl_output::WlOutput>,
    pub(super) focused_view_title: String,
}

impl RiverState {
    pub(super) fn new(listeners: SnapshotListeners, latest: LatestSnapshotsStore) -> Self {
        Self {
            listeners,
            latest,
            manager: None,
            seat: None,
            seat_status: None,
            output_statuses: HashMap::new(),
            focused_output: None,
            last_updated_output: None,
            output_order: Vec::new(),
            output_global_name_by_id: HashMap::new(),
            output_name_by_id: HashMap::new(),
            output_states: HashMap::new(),
            output_view_tags: HashMap::new(),
            output_by_global_name: HashMap::new(),
            focused_view_title: String::new(),
        }
    }

    pub(super) fn emit_snapshot(&mut self) {
        let focused = self
            .focused_output
            .as_ref()
            .or(self.last_updated_output.as_ref());
        self.refresh_latest(focused);

        if let Ok(mut listeners) = self.listeners.lock() {
            listeners.retain_mut(|listener| {
                let snapshot = self.snapshot_for_selector(listener.selector.as_deref(), focused);
                if listener.last_sent.as_ref() == Some(&snapshot) {
                    return true;
                }
                if listener.tx.try_send(snapshot.clone()).is_ok() {
                    listener.last_sent = Some(snapshot);
                    true
                } else {
                    false
                }
            });
        }
    }

    pub(super) fn bind_initial(
        &mut self,
        globals: &GlobalList,
        qh: &QueueHandle<Self>,
    ) -> Result<(), String> {
        let manager: zriver_status_manager_v1::ZriverStatusManagerV1 = globals
            .bind(qh, 1..=4, ())
            .map_err(|err| format!("bind manager failed: {}", err))?;
        self.manager = Some(manager);

        let snapshot = globals.contents().clone_list();
        let registry = globals.registry();

        for global in &snapshot {
            if global.interface == wl_output::WlOutput::interface().name {
                self.bind_output(registry, global.name, global.version, qh)?;
            } else if global.interface == wl_seat::WlSeat::interface().name
                && self.seat_status.is_none()
            {
                self.bind_seat(registry, global.name, global.version, qh)?;
            }
        }

        Ok(())
    }

    pub(super) fn bind_output(
        &mut self,
        registry: &wl_registry::WlRegistry,
        name: u32,
        version: u32,
        qh: &QueueHandle<Self>,
    ) -> Result<(), String> {
        let output: wl_output::WlOutput = registry.bind(name, version.min(4), qh, name);
        self.output_by_global_name.insert(name, output.clone());
        self.output_order.push(output.id());
        self.output_global_name_by_id.insert(output.id(), name);
        self.output_states.entry(output.id()).or_default();
        self.output_view_tags.entry(output.id()).or_default();

        if let Some(manager) = self.manager.as_ref() {
            let status = manager.get_river_output_status(&output, qh, output.id());
            self.output_statuses.insert(output.id(), status);
        }

        Ok(())
    }

    pub(super) fn bind_seat(
        &mut self,
        registry: &wl_registry::WlRegistry,
        name: u32,
        version: u32,
        qh: &QueueHandle<Self>,
    ) -> Result<(), String> {
        let seat: wl_seat::WlSeat = registry.bind(name, version.min(3), qh, ());
        self.seat = Some(seat.clone());
        if let Some(manager) = self.manager.as_ref() {
            let status = manager.get_river_seat_status(&seat, qh, ());
            self.seat_status = Some(status);
        }
        Ok(())
    }

    fn refresh_latest(&self, focused: Option<&wayland_client::backend::ObjectId>) {
        let mut latest = match self.latest.lock() {
            Ok(v) => v,
            Err(_) => return,
        };
        latest.by_index.clear();
        latest.by_global.clear();
        latest.by_name.clear();
        latest.focused_output_name = focused.and_then(|id| self.output_name_by_id.get(id).cloned());

        for id in &self.output_order {
            let snapshot = self.snapshot_for_selector(None, Some(id));
            latest.by_index.push(snapshot.clone());
            if let Some(global) = self.output_global_name_by_id.get(id) {
                latest.by_global.insert(*global, snapshot.clone());
            }
            if let Some(name) = self.output_name_by_id.get(id) {
                latest.by_name.insert(name.to_lowercase(), snapshot);
            }
        }
        latest.default = self.snapshot_for_selector(None, focused);
    }

    pub(super) fn snapshot_for_selector(
        &self,
        selector: Option<&str>,
        focused: Option<&wayland_client::backend::ObjectId>,
    ) -> RiverSnapshot {
        let source_output = match selector.and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("focused") {
                return None;
            }
            if let Ok(index) = trimmed.parse::<usize>() {
                return self.output_order.get(index);
            }
            if let Ok(global_name) = trimmed.parse::<u32>() {
                return self
                    .output_global_name_by_id
                    .iter()
                    .find_map(|(id, g)| if *g == global_name { Some(id) } else { None });
            }
            let target = trimmed.to_lowercase();
            self.output_name_by_id
                .iter()
                .find_map(|(id, name)| (name.to_lowercase() == target).then_some(id))
                .or_else(|| {
                    self.output_name_by_id
                        .iter()
                        .find_map(|(id, name)| name.to_lowercase().contains(&target).then_some(id))
                })
        }) {
            Some(id) => Some(id),
            None => focused,
        };

        let Some(output_id) = source_output else {
            return RiverSnapshot::default();
        };
        let Some(state) = self.output_states.get(output_id) else {
            return RiverSnapshot::default();
        };
        let view_tags = self
            .output_view_tags
            .get(output_id)
            .cloned()
            .unwrap_or_default();
        let focused_title = if self.focused_output.as_ref() == Some(output_id) {
            self.focused_view_title.clone()
        } else {
            String::new()
        };

        RiverSnapshot {
            focused: state.focused,
            occupied: state.occupied,
            urgent: state.urgent,
            focused_title,
            view_tags,
        }
    }
}
