use wayland_client::globals::GlobalListContents;
use wayland_client::protocol::wl_registry;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use crate::protocols::ext_foreign_toplevel_list::ext_foreign_toplevel_list_v1::{
    ext_foreign_toplevel_handle_v1, ext_foreign_toplevel_list_v1,
};

use super::state::ToplevelState;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for ToplevelState {
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

impl Dispatch<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, ()> for ToplevelState {
    fn event(
        state: &mut Self,
        _: &ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
        event: ext_foreign_toplevel_list_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } => {
                let id = toplevel.id();
                if !state.handles.contains_key(&id) {
                    state.order.push(id.clone());
                }
                state.handles.entry(id).or_default();
                state.emit_updates();
            }
            ext_foreign_toplevel_list_v1::Event::Finished => {}
        }
    }

    wayland_client::event_created_child!(
        ToplevelState,
        ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
        [
            ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (
                ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
                ()
            )
        ]
    );
}

impl Dispatch<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ()> for ToplevelState {
    fn event(
        state: &mut Self,
        handle: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let info = state.handles.entry(handle.id()).or_default();

        match event {
            ext_foreign_toplevel_handle_v1::Event::Title { title } => {
                info.title = title;
            }
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                info.app_id = app_id;
            }
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                info.identifier = identifier;
            }
            ext_foreign_toplevel_handle_v1::Event::Closed => {
                let id = handle.id();
                state.handles.remove(&id);
                state.order.retain(|entry| entry != &id);
            }
            ext_foreign_toplevel_handle_v1::Event::Done => {}
        }

        state.emit_updates();
    }
}
