use wayland_client::globals::GlobalListContents;
use wayland_client::protocol::{wl_output, wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use crate::protocols::river_status::river_status_unstable_v1::{
    zriver_output_status_v1, zriver_seat_status_v1, zriver_status_manager_v1,
};

use super::model::{parse_view_tags, union_view_tags};
use super::state::RiverState;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for RiverState {
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
                    if let Err(err) = state.bind_output(registry, name, version, qh) {
                        log::warn!("river-status: failed to bind output: {}", err);
                    }
                } else if interface == wl_seat::WlSeat::interface().name
                    && state.seat_status.is_none()
                    && let Err(err) = state.bind_seat(registry, name, version, qh)
                {
                    log::warn!("river-status: failed to bind seat: {}", err);
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(output) = state.output_by_global_name.remove(&name) {
                    let id = output.id();
                    state.output_states.remove(&id);
                    state.output_statuses.remove(&id);
                    state.output_order.retain(|v| v != &id);
                    state.output_global_name_by_id.remove(&id);
                    state.output_name_by_id.remove(&id);
                    state.output_view_tags.remove(&id);
                    if state.focused_output == Some(id.clone()) {
                        state.focused_output = None;
                    }
                    if state.last_updated_output == Some(id) {
                        state.last_updated_output = None;
                    }
                    state.emit_snapshot();
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, u32> for RiverState {
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
            state.emit_snapshot();
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for RiverState {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zriver_status_manager_v1::ZriverStatusManagerV1, ()> for RiverState {
    fn event(
        _: &mut Self,
        _: &zriver_status_manager_v1::ZriverStatusManagerV1,
        _: zriver_status_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zriver_seat_status_v1::ZriverSeatStatusV1, ()> for RiverState {
    fn event(
        state: &mut Self,
        _: &zriver_seat_status_v1::ZriverSeatStatusV1,
        event: zriver_seat_status_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zriver_seat_status_v1::Event::FocusedOutput { output } => {
                state.focused_output = Some(output.id());
                state.emit_snapshot();
            }
            zriver_seat_status_v1::Event::UnfocusedOutput { output } => {
                if state.focused_output == Some(output.id()) {
                    state.focused_output = None;
                    state.emit_snapshot();
                }
            }
            zriver_seat_status_v1::Event::FocusedView { title } => {
                state.focused_view_title = title;
                state.emit_snapshot();
            }
            _ => {}
        }
    }
}

impl Dispatch<zriver_output_status_v1::ZriverOutputStatusV1, wayland_client::backend::ObjectId>
    for RiverState
{
    fn event(
        state: &mut Self,
        _: &zriver_output_status_v1::ZriverOutputStatusV1,
        event: zriver_output_status_v1::Event,
        output_id: &wayland_client::backend::ObjectId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let entry = state.output_states.entry(output_id.clone()).or_default();
        state.last_updated_output = Some(output_id.clone());
        match event {
            zriver_output_status_v1::Event::FocusedTags { tags } => {
                entry.focused = tags;
                state.emit_snapshot();
            }
            zriver_output_status_v1::Event::ViewTags { tags } => {
                let parsed = parse_view_tags(&tags);
                entry.occupied = union_view_tags(&parsed);
                state.output_view_tags.insert(output_id.clone(), parsed);
                state.emit_snapshot();
            }
            zriver_output_status_v1::Event::UrgentTags { tags } => {
                entry.urgent = tags;
                state.emit_snapshot();
            }
            _ => {}
        }
    }
}
