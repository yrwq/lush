use mlua::{Lua, Result};

use crate::runtime::lua_runtime::LuaStateBridge;

pub fn install(lua: &Lua, bridge: LuaStateBridge) -> Result<()> {
    let get_bridge = bridge.clone();
    lua.globals().set(
        "_lush_get",
        lua.create_function(move |_, name: String| Ok(get_bridge.get(&name)))?,
    )?;

    let set_bridge = bridge.clone();
    lua.globals().set(
        "_lush_set",
        lua.create_function(move |_, (name, value): (String, String)| {
            set_bridge.set(&name, &value);
            Ok(())
        })?,
    )?;

    lua.globals().set(
        "_lush_snapshot",
        lua.create_function(move |lua, (): ()| {
            let out = lua.create_table()?;
            for (k, v) in bridge.snapshot() {
                out.set(k, v)?;
            }
            Ok(out)
        })?,
    )?;

    Ok(())
}
