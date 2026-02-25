use wayland_client::globals::{GlobalList, GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use crate::protocols::river_control::river_control_unstable_v1::{
    zriver_command_callback_v1, zriver_control_v1,
};

#[derive(Default)]
struct RiverControlState {
    result: Option<Result<(), String>>,
}

pub(super) fn focus_workspace(index_1_based: u32) -> Result<(), String> {
    let mask = 1_u32 << (index_1_based - 1);
    run_control_command(["set-focused-tags".to_string(), mask.to_string()])
}

pub(super) fn focus_view_tags() -> Result<(), String> {
    run_control_command(["focus-view-tags".to_string()])
}

fn run_control_command<const N: usize>(args: [String; N]) -> Result<(), String> {
    let conn = Connection::connect_to_env()
        .map_err(|err| format!("river control wayland connect failed: {}", err))?;
    let (globals, mut event_queue) = registry_queue_init::<RiverControlState>(&conn)
        .map_err(|err| format!("river control registry init failed: {}", err))?;
    let qh = event_queue.handle();
    let mut state = RiverControlState::default();

    let control = bind_river_control(&globals, &qh)?;
    let seat = bind_first_seat(&globals, &qh)?;

    for arg in args {
        control.add_argument(arg);
    }
    let _cb = control.run_command(&seat, &qh, ());

    loop {
        if event_queue.blocking_dispatch(&mut state).is_err() {
            return Err("river control dispatch failed".to_string());
        }
        if let Some(result) = state.result.take() {
            return result;
        }
    }
}

fn bind_river_control(
    globals: &GlobalList,
    qh: &QueueHandle<RiverControlState>,
) -> Result<zriver_control_v1::ZriverControlV1, String> {
    globals
        .bind(qh, 1..=1, ())
        .map_err(|err| format!("bind zriver_control_v1 failed: {}", err))
}

fn bind_first_seat(
    globals: &GlobalList,
    qh: &QueueHandle<RiverControlState>,
) -> Result<wl_seat::WlSeat, String> {
    let snapshot = globals.contents().clone_list();
    let registry = globals.registry();

    let seat_global = snapshot
        .iter()
        .find(|g| g.interface == wl_seat::WlSeat::interface().name)
        .ok_or_else(|| "no wl_seat global available".to_string())?;

    Ok(registry.bind(seat_global.name, seat_global.version.min(3), qh, ()))
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for RiverControlState {
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

impl Dispatch<wl_seat::WlSeat, ()> for RiverControlState {
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

impl Dispatch<zriver_control_v1::ZriverControlV1, ()> for RiverControlState {
    fn event(
        _: &mut Self,
        _: &zriver_control_v1::ZriverControlV1,
        _: zriver_control_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zriver_command_callback_v1::ZriverCommandCallbackV1, ()> for RiverControlState {
    fn event(
        state: &mut Self,
        _: &zriver_command_callback_v1::ZriverCommandCallbackV1,
        event: zriver_command_callback_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zriver_command_callback_v1::Event::Success { .. } => {
                state.result = Some(Ok(()));
            }
            zriver_command_callback_v1::Event::Failure { failure_message } => {
                state.result = Some(Err(format!("river control failure: {}", failure_message)));
            }
        }
    }
}
