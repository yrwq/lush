use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use mlua::{Function, Lua, Result};

pub fn install(lua: &Lua) -> Result<()> {
    let timer_seq = Rc::new(AtomicU64::new(1));
    let timers: Rc<RefCell<HashMap<u64, glib::SourceId>>> = Rc::new(RefCell::new(HashMap::new()));

    let every_seq = timer_seq.clone();
    let every_timers = timers.clone();
    lua.globals().set(
        "_lush_scheduler_every",
        lua.create_function(move |_, (interval_ms, callback): (u64, Function)| {
            let id = every_seq.fetch_add(1, Ordering::Relaxed);
            let source =
                glib::timeout_add_local(Duration::from_millis(interval_ms.max(1)), move || {
                    if let Err(err) = callback.call::<()>(()) {
                        log::warn!("lush timer callback failed: {}", err);
                    }
                    glib::ControlFlow::Continue
                });
            every_timers.borrow_mut().insert(id, source);
            Ok(id)
        })?,
    )?;

    let after_seq = timer_seq.clone();
    let after_timers = timers.clone();
    lua.globals().set(
        "_lush_scheduler_after",
        lua.create_function(move |_, (delay_ms, callback): (u64, Function)| {
            let id = after_seq.fetch_add(1, Ordering::Relaxed);
            let timers_ref = after_timers.clone();
            let source =
                glib::timeout_add_local(Duration::from_millis(delay_ms.max(1)), move || {
                    timers_ref.borrow_mut().remove(&id);
                    if let Err(err) = callback.call::<()>(()) {
                        log::warn!("lush timer callback failed: {}", err);
                    }
                    glib::ControlFlow::Break
                });
            after_timers.borrow_mut().insert(id, source);
            Ok(id)
        })?,
    )?;

    lua.globals().set(
        "_lush_scheduler_cancel",
        lua.create_function(move |_, id: u64| {
            if let Some(source) = timers.borrow_mut().remove(&id) {
                source.remove();
            }
            Ok(())
        })?,
    )?;

    Ok(())
}
