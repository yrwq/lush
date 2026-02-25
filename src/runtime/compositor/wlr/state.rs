use std::collections::HashMap;

use wayland_client::protocol::wl_seat;

use crate::protocols::wlr_foreign_toplevel_management::wlr_foreign_toplevel_management_unstable_v1::zwlr_foreign_toplevel_handle_v1;

#[derive(Clone, Debug, Default)]
pub(super) struct HandleInfo {
    pub(super) title: String,
    pub(super) app_id: String,
    pub(super) handle: Option<zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1>,
}

#[derive(Default)]
pub(super) struct WlrActionState {
    pub(super) seat: Option<wl_seat::WlSeat>,
    pub(super) entries: HashMap<wayland_client::backend::ObjectId, HandleInfo>,
    pub(super) order: Vec<wayland_client::backend::ObjectId>,
}
