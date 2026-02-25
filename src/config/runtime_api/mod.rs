use mlua::{Lua, Result};

use crate::runtime::lua_runtime::LuaStateBridge;

mod audio;
mod data;
mod lua_modules;
mod notifications;
mod process;
mod scheduler;
mod state;
mod ui;
mod windows;

pub fn install_lush_api(lua: &Lua, bridge: LuaStateBridge) -> Result<()> {
    state::install(lua, bridge.clone())?;
    data::install(lua, bridge.clone())?;
    audio::install(lua)?;
    windows::install(lua, bridge.clone())?;
    notifications::install(lua, bridge.clone())?;
    process::install(lua)?;
    scheduler::install(lua)?;
    Ok(())
}

pub fn install_ui_api(lua: &Lua, cfg_global: &'static str) -> Result<()> {
    ui::install(lua, cfg_global)
}

pub fn load_lush_lua_modules(lua: &Lua) -> Result<()> {
    lua_modules::load(lua)
}
