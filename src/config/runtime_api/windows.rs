use mlua::{Lua, Result};

use crate::runtime::lua_runtime::AppCommand;
use crate::runtime::lua_runtime::LuaStateBridge;

pub fn install(lua: &Lua, bridge: LuaStateBridge) -> Result<()> {
    let open_bridge = bridge.clone();
    lua.globals().set(
        "_lush_window_open",
        lua.create_function(move |_, name: String| {
            open_bridge.queue_app_command(AppCommand::Open(name));
            Ok(())
        })?,
    )?;

    let close_bridge = bridge.clone();
    lua.globals().set(
        "_lush_window_close",
        lua.create_function(move |_, name: String| {
            close_bridge.queue_app_command(AppCommand::Close(name));
            Ok(())
        })?,
    )?;

    let toggle_bridge = bridge.clone();
    lua.globals().set(
        "_lush_window_toggle",
        lua.create_function(move |_, name: String| {
            toggle_bridge.queue_app_command(AppCommand::Toggle(name));
            Ok(())
        })?,
    )?;

    let visible_bridge = bridge.clone();
    lua.globals().set(
        "_lush_window_set_visible",
        lua.create_function(move |_, (name, visible): (String, bool)| {
            visible_bridge.queue_app_command(AppCommand::SetVisible(name, visible));
            Ok(())
        })?,
    )?;

    let is_visible_bridge = bridge.clone();
    lua.globals().set(
        "_lush_window_is_visible",
        lua.create_function(move |_, name: String| Ok(is_visible_bridge.is_window_visible(&name)))?,
    )?;

    lua.globals().set(
        "_lush_window_list",
        lua.create_function(move |lua, (): ()| {
            let out = lua.create_table()?;
            for (idx, name) in bridge.window_names().into_iter().enumerate() {
                out.set(idx + 1, name)?;
            }
            Ok(out)
        })?,
    )?;

    Ok(())
}
