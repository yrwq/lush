use mlua::{Lua, Result};

use crate::runtime::lua_runtime::LuaStateBridge;
use crate::runtime::notifications;

pub fn install(lua: &Lua, bridge: LuaStateBridge) -> Result<()> {
    let send_bridge = bridge.clone();
    lua.globals().set(
        "_lush_notification_send",
        lua.create_function(
            move |_,
                  (title, body, icon, urgency, timeout_ms): (
                String,
                String,
                String,
                String,
                i32,
            )| {
                if let Some(()) = send_bridge.with_bus(|bus| {
                    notifications::send(
                        bus,
                        &title,
                        &body,
                        Some(&icon),
                        Some(&urgency),
                        Some(timeout_ms),
                    );
                }) {
                    Ok(())
                } else {
                    Err(mlua::Error::runtime(
                        "lush.notifications.send called before app runtime was ready",
                    ))
                }
            },
        )?,
    )?;

    let clear_bridge = bridge.clone();
    lua.globals().set(
        "_lush_notification_clear",
        lua.create_function(move |_, (): ()| {
            if let Some(()) = clear_bridge.with_bus(notifications::clear_history_now) {
                Ok(())
            } else {
                Err(mlua::Error::runtime(
                    "lush.notifications.clear called before app runtime was ready",
                ))
            }
        })?,
    )?;

    lua.globals().set(
        "_lush_notification_delete",
        lua.create_function(move |_, index: usize| {
            if let Some(()) = bridge.with_bus(|bus| notifications::delete_history_now(bus, index)) {
                Ok(())
            } else {
                Err(mlua::Error::runtime(
                    "lush.notifications.delete called before app runtime was ready",
                ))
            }
        })?,
    )?;

    Ok(())
}
