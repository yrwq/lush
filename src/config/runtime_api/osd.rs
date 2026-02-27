use mlua::{Lua, Result, Table, Value};

use crate::runtime::lua_runtime::LuaStateBridge;

pub fn install(lua: &Lua, bridge: LuaStateBridge) -> Result<()> {
    let osd_bridge = bridge.clone();
    lua.globals().set(
        "_lush_osd_bind",
        lua.create_function(
            move |_, (name, signals, timeout_ms): (String, Value, Option<u64>)| {
                let signals = parse_signals(signals)?;
                if let Err(err) = osd_bridge.osd_bind(&name, signals, timeout_ms.unwrap_or(1200)) {
                    log::warn!("lush osd bind failed: {}", err);
                }
                Ok(())
            },
        )?,
    )?;

    Ok(())
}

fn parse_signals(value: Value) -> Result<Vec<String>> {
    match value {
        Value::String(s) => Ok(vec![s.to_str()?.to_string()]),
        Value::Table(t) => parse_signals_table(t),
        Value::Nil => Ok(Vec::new()),
        _ => Ok(Vec::new()),
    }
}

fn parse_signals_table(t: Table) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for item in t.sequence_values::<Value>() {
        match item? {
            Value::String(s) => out.push(s.to_str()?.to_string()),
            Value::Integer(i) => out.push(i.to_string()),
            Value::Number(n) => out.push(n.to_string()),
            _ => {}
        }
    }
    Ok(out)
}
