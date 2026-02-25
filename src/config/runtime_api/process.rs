use std::process::Command;

use glib::MainContext;
use mlua::{Function, Lua, Result};

pub fn install(lua: &Lua) -> Result<()> {
    lua.globals().set(
        "_lush_process_capture",
        lua.create_function(move |_, command: String| {
            let output = Command::new("sh").arg("-c").arg(&command).output();
            let text = match output {
                Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
                Err(_) => String::new(),
            };
            Ok(text)
        })?,
    )?;

    lua.globals().set(
        "_lush_process_spawn",
        lua.create_function(move |_, command: String| {
            let ok = Command::new("sh").arg("-c").arg(&command).spawn().is_ok();
            Ok(ok)
        })?,
    )?;

    lua.globals().set(
        "_lush_process_capture_async",
        lua.create_function(move |_, (command, callback): (String, Function)| {
            let (tx, rx) = async_channel::bounded::<String>(1);
            std::thread::spawn(move || {
                let output = Command::new("sh").arg("-c").arg(&command).output();
                let text = match output {
                    Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
                    Err(_) => String::new(),
                };
                let _ = tx.send_blocking(text);
            });

            MainContext::default().spawn_local(async move {
                if let Ok(text) = rx.recv().await
                    && let Err(err) = callback.call::<()>(text)
                {
                    log::warn!("lush process async callback failed: {}", err);
                }
            });
            Ok(())
        })?,
    )?;

    Ok(())
}
