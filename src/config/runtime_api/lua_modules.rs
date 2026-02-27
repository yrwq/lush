use mlua::{Lua, Result};

const LUSH_LUA_ENTRYPOINT: &str = include_str!("../../../lua/lush.lua");
const LUSH_LUA_EVENTS: &str = include_str!("../../../lua/lush/events.lua");
const LUSH_LUA_STATE: &str = include_str!("../../../lua/lush/state.lua");
const LUSH_LUA_SIGNAL: &str = include_str!("../../../lua/lush/signal.lua");
const LUSH_LUA_WINDOWS: &str = include_str!("../../../lua/lush/windows.lua");
const LUSH_LUA_NOTIFICATIONS: &str = include_str!("../../../lua/lush/notifications.lua");
const LUSH_LUA_OSD: &str = include_str!("../../../lua/lush/osd.lua");
const LUSH_LUA_SCHEDULER: &str = include_str!("../../../lua/lush/scheduler.lua");
const LUSH_LUA_PROCESS: &str = include_str!("../../../lua/lush/process.lua");
const LUSH_LUA_AUDIO: &str = include_str!("../../../lua/lush/audio.lua");
const LUSH_LUA_DATA: &str = include_str!("../../../lua/lush/data.lua");
const LUSH_LUA_UI: &str = include_str!("../../../lua/lush/ui.lua");
const LUSH_LUA_STORE: &str = include_str!("../../../lua/lush/store.lua");
const LUSH_LUA_INIT: &str = include_str!("../../../lua/lush/init.lua");

pub fn load(lua: &Lua) -> Result<()> {
    let modules = [
        ("lush/events.lua", LUSH_LUA_EVENTS),
        ("lush/state.lua", LUSH_LUA_STATE),
        ("lush/signal.lua", LUSH_LUA_SIGNAL),
        ("lush/windows.lua", LUSH_LUA_WINDOWS),
        ("lush/notifications.lua", LUSH_LUA_NOTIFICATIONS),
        ("lush/osd.lua", LUSH_LUA_OSD),
        ("lush/scheduler.lua", LUSH_LUA_SCHEDULER),
        ("lush/process.lua", LUSH_LUA_PROCESS),
        ("lush/audio.lua", LUSH_LUA_AUDIO),
        ("lush/data.lua", LUSH_LUA_DATA),
        ("lush/ui.lua", LUSH_LUA_UI),
        ("lush/store.lua", LUSH_LUA_STORE),
        ("lush/init.lua", LUSH_LUA_INIT),
        ("lush.lua", LUSH_LUA_ENTRYPOINT),
    ];
    for (name, source) in modules {
        lua.load(source).set_name(name).exec()?;
    }
    Ok(())
}
