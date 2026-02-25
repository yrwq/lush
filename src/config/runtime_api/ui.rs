use mlua::{Lua, Result, Table};

pub fn install(lua: &Lua, cfg_global: &'static str) -> Result<()> {
    lua.globals().set(
        "_lush_ui_set_windows",
        lua.create_function(move |lua, list: Table| {
            lua.globals()
                .get::<Table>(cfg_global)?
                .set("windows", list)?;
            Ok(())
        })?,
    )?;
    lua.globals().set(
        "_lush_ui_set_css",
        lua.create_function(move |lua, value: String| {
            lua.globals().get::<Table>(cfg_global)?.set("css", value)?;
            Ok(())
        })?,
    )?;
    Ok(())
}
