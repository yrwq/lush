use wayland_client::globals::GlobalListContents;
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use crate::protocols::wlr_foreign_toplevel_management::wlr_foreign_toplevel_management_unstable_v1::{
    zwlr_foreign_toplevel_handle_v1, zwlr_foreign_toplevel_manager_v1,
};

use super::state::{HandleInfo, WlrActionState};

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for WlrActionState {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WlrActionState {
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

impl Dispatch<zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1, ()>
    for WlrActionState
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
                if !state.entries.contains_key(&id) {
                    state.order.push(id.clone());
                }
                state.entries.entry(id).or_insert_with(|| HandleInfo {
                    handle: Some(toplevel),
                    ..HandleInfo::default()
                });
            }
            zwlr_foreign_toplevel_manager_v1::Event::Finished => {}
        }
    }

    wayland_client::event_created_child!(
        WlrActionState,
        zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
        [
            zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (
                zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
                ()
            )
        ]
    );
}

impl Dispatch<zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1, ()> for WlrActionState {
    fn event(
        state: &mut Self,
        handle: &zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let entry = state
            .entries
            .entry(handle.id())
            .or_insert_with(|| HandleInfo {
                handle: Some(handle.clone()),
                ..HandleInfo::default()
            });

        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                entry.title = title;
            }
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                entry.app_id = app_id;
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                let id = handle.id();
                state.entries.remove(&id);
                state.order.retain(|entry_id| entry_id != &id);
            }
            _ => {}
        }
    }
}
