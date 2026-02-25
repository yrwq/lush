use std::path::PathBuf;
use std::rc::Rc;

use mlua::{Lua, Result, Table};

use crate::runtime::lua_runtime::{LuaRuntime, LuaStateBridge};

use super::parser::{WIDGET_KINDS, parse_windows, widget_ctor};
use super::runtime_api;
use super::types::{AppConfig, LoadedConfig};

const GLOBAL_CFG: &str = "__cfg";
const GLOBAL_CB_SEQ: &str = "__cb_seq";
const GLOBAL_CALLBACKS: &str = "__callbacks";

pub fn load(path: &PathBuf) -> Result<LoadedConfig> {
    let lua = Lua::new();
    let bridge = LuaStateBridge::default();
    initialize_lua_state(&lua, bridge.clone())?;
    run_config_script(&lua, path)?;

    let cfg: Table = lua.globals().get(GLOBAL_CFG)?;
    let windows = parse_windows(&lua, &cfg)?;
    let css = cfg.get("css").ok();

    Ok(LoadedConfig {
        app: AppConfig { css, windows },
        runtime: Rc::new(LuaRuntime::new(lua, bridge)),
    })
}

fn initialize_lua_state(lua: &Lua, bridge: LuaStateBridge) -> Result<()> {
    lua.globals().set(GLOBAL_CFG, lua.create_table()?)?;
    lua.globals().set(GLOBAL_CB_SEQ, 0_u64)?;
    lua.globals().set(GLOBAL_CALLBACKS, lua.create_table()?)?;

    install_widget_constructors(lua)?;
    runtime_api::install_lush_api(lua, bridge)?;
    runtime_api::install_ui_api(lua, GLOBAL_CFG)?;
    runtime_api::load_lush_lua_modules(lua)?;
    Ok(())
}

fn install_widget_constructors(lua: &Lua) -> Result<()> {
    let ctors = lua.create_table()?;
    for kind in WIDGET_KINDS {
        let alias = kind.replace('-', "_");
        ctors.set(alias, widget_ctor(lua, kind)?)?;
    }
    lua.globals().set("_lush_widget_ctors", ctors)?;
    Ok(())
}

fn run_config_script(lua: &Lua, path: &PathBuf) -> Result<()> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| mlua::Error::runtime(format!("cannot read {:?}: {}", path, e)))?;
    lua.load(&source).set_name(path.to_string_lossy()).exec()
}

pub fn find_config() -> PathBuf {
    let candidates = [
        std::env::var("LUSH_CONFIG").ok().map(PathBuf::from),
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(|p| PathBuf::from(p).join("lush").join("config.lua")),
        std::env::var("HOME").ok().map(|p| {
            PathBuf::from(p)
                .join(".config")
                .join("lush")
                .join("config.lua")
        }),
        Some(PathBuf::from("config.lua")),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("config.lua"))
}
