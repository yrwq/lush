use mlua::{Lua, Result};

use crate::runtime::audio_control;

pub fn install(lua: &Lua) -> Result<()> {
    lua.globals().set(
        "_lush_audio_set_volume",
        lua.create_function(move |_, percent: u32| {
            match audio_control::set_default_sink_volume_percent(percent) {
                Ok(()) => Ok(true),
                Err(err) => {
                    log::warn!("lush audio set volume failed: {}", err);
                    Ok(false)
                }
            }
        })?,
    )?;

    lua.globals().set(
        "_lush_audio_toggle_mute",
        lua.create_function(
            move |_, (): ()| match audio_control::toggle_default_sink_mute() {
                Ok(()) => Ok(true),
                Err(err) => {
                    log::warn!("lush audio toggle mute failed: {}", err);
                    Ok(false)
                }
            },
        )?,
    )?;

    lua.globals().set(
        "_lush_audio_set_muted",
        lua.create_function(move |_, muted: bool| {
            match audio_control::set_default_sink_mute(muted) {
                Ok(()) => Ok(true),
                Err(err) => {
                    log::warn!("lush audio set muted failed: {}", err);
                    Ok(false)
                }
            }
        })?,
    )?;

    Ok(())
}
