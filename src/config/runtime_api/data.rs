use mlua::{Lua, Result, Table, Value};

use crate::runtime::lua_runtime::LuaStateBridge;

pub fn install(lua: &Lua, bridge: LuaStateBridge) -> Result<()> {
    let use_bridge = bridge.clone();
    lua.globals().set(
        "_lush_data_use",
        lua.create_function(move |_, (name, opts): (String, Option<Table>)| {
            let interval = opts
                .as_ref()
                .and_then(|t| t.get::<Option<u32>>("interval").ok())
                .flatten();
            let iface = opts
                .as_ref()
                .and_then(|t| t.get::<Option<String>>("iface").ok())
                .flatten();
            let path = opts
                .as_ref()
                .and_then(|t| t.get::<Option<String>>("path").ok())
                .flatten();
            let output = opts.as_ref().and_then(|t| match t.get::<Value>("output") {
                Ok(Value::String(s)) => s.to_str().ok().map(|v| v.to_string()),
                Ok(Value::Integer(i)) if i >= 0 => Some(i.to_string()),
                Ok(Value::Number(n)) if n >= 0.0 && n.fract() == 0.0 => {
                    Some((n as u64).to_string())
                }
                _ => None,
            });
            use_bridge
                .data_use(&name, interval, iface, path, output)
                .map_err(mlua::Error::runtime)
        })?,
    )?;

    lua.globals().set(
        "_lush_data_unuse",
        lua.create_function(move |_, name: String| {
            bridge.data_unuse(&name).map_err(mlua::Error::runtime)
        })?,
    )?;

    Ok(())
}
