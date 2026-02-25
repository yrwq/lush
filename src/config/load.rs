use std::path::{Path, PathBuf};
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
    initialize_lua_state(&lua, bridge.clone(), path)?;
    run_config_script(&lua, path)?;

    let cfg: Table = lua.globals().get(GLOBAL_CFG)?;
    let windows = parse_windows(&lua, &cfg)?;
    let css = cfg
        .get::<String>("css")
        .ok()
        .map(|raw| resolve_config_relative_path(path, &raw));

    Ok(LoadedConfig {
        app: AppConfig { css, windows },
        runtime: Rc::new(LuaRuntime::new(lua, bridge)),
    })
}

fn resolve_config_relative_path(config_path: &Path, raw: &str) -> String {
    let css_path = PathBuf::from(raw);
    if css_path.is_absolute() {
        return raw.to_string();
    }

    if css_path.exists() {
        return css_path.to_string_lossy().to_string();
    }

    let joined = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(css_path)
        .to_string_lossy()
        .to_string();

    joined
}

fn initialize_lua_state(lua: &Lua, bridge: LuaStateBridge, config_path: &Path) -> Result<()> {
    lua.globals().set(GLOBAL_CFG, lua.create_table()?)?;
    lua.globals().set(GLOBAL_CB_SEQ, 0_u64)?;
    lua.globals().set(GLOBAL_CALLBACKS, lua.create_table()?)?;

    configure_package_path(lua, config_path)?;
    install_widget_constructors(lua)?;
    runtime_api::install_lush_api(lua, bridge)?;
    runtime_api::install_ui_api(lua, GLOBAL_CFG)?;
    runtime_api::load_lush_lua_modules(lua)?;
    Ok(())
}

fn configure_package_path(lua: &Lua, config_path: &Path) -> Result<()> {
    let config_dir = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_string_lossy()
        .replace('\\', "/");
    let prepended = format!("{config_dir}/?.lua;{config_dir}/?/init.lua");
    let package: Table = lua.globals().get("package")?;
    let existing: String = package.get("path")?;
    let merged = if existing.is_empty() {
        prepended
    } else {
        format!("{prepended};{existing}")
    };
    package.set("path", merged)?;
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
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(raw) = std::env::var("LUSH_CONFIG").ok() {
        let path = PathBuf::from(raw);
        if path.extension().is_some() {
            candidates.push(path);
        } else {
            candidates.push(path.join("init.lua"));
        }
    }

    if let Some(base) = std::env::var("XDG_CONFIG_HOME").ok() {
        let dir = PathBuf::from(base).join("lush");
        candidates.push(dir.join("init.lua"));
    }

    if let Some(home) = std::env::var("HOME").ok() {
        let dir = PathBuf::from(home).join(".config").join("lush");
        candidates.push(dir.join("init.lua"));
    }

    candidates.push(PathBuf::from("init.lua"));

    candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .unwrap_or_else(|| PathBuf::from("init.lua"))
}
