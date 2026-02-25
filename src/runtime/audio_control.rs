use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::time::{Duration, Instant};

use libpulse_binding as pulse;
use pulse::callbacks::ListResult;
use pulse::context::{Context, FlagSet as ContextFlagSet, State as ContextState};
use pulse::mainloop::standard::{IterateResult, Mainloop};
use pulse::proplist::Proplist;
use pulse::volume::{ChannelVolumes, Volume};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(2000);
const QUERY_TIMEOUT: Duration = Duration::from_millis(1500);
const APPLY_TIMEOUT: Duration = Duration::from_millis(1500);
const MAX_PERCENT: u32 = 150;

#[derive(Clone, Copy)]
struct SinkState {
    volume: ChannelVolumes,
    muted: bool,
}

pub fn set_default_sink_volume_percent(percent: u32) -> Result<(), String> {
    with_context(|mainloop, context| {
        let sink_name = query_default_sink_name(mainloop, context)?;
        let sink_state = query_sink_state(mainloop, context, &sink_name)?;
        let mut next = sink_state.volume;
        let clamped = percent.min(MAX_PERCENT);
        let raw = ((clamped as f64 / 100.0) * (Volume::NORMAL.0 as f64)).round();
        next.set(next.len(), Volume(raw as u32));
        apply_sink_volume(mainloop, context, &sink_name, &next)
    })
}

pub fn toggle_default_sink_mute() -> Result<(), String> {
    with_context(|mainloop, context| {
        let sink_name = query_default_sink_name(mainloop, context)?;
        let sink_state = query_sink_state(mainloop, context, &sink_name)?;
        apply_sink_mute(mainloop, context, &sink_name, !sink_state.muted)
    })
}

pub fn set_default_sink_mute(muted: bool) -> Result<(), String> {
    with_context(|mainloop, context| {
        let sink_name = query_default_sink_name(mainloop, context)?;
        apply_sink_mute(mainloop, context, &sink_name, muted)
    })
}

fn with_context<T, F>(f: F) -> Result<T, String>
where
    F: FnOnce(&Rc<RefCell<Mainloop>>, &Rc<RefCell<Context>>) -> Result<T, String>,
{
    let mut proplist =
        Proplist::new().ok_or_else(|| "audio control: failed to create proplist".to_string())?;
    let _ = proplist.set_str(pulse::proplist::properties::APPLICATION_NAME, "lush");

    let mainloop =
        Rc::new(RefCell::new(Mainloop::new().ok_or_else(|| {
            "audio control: failed to create mainloop".to_string()
        })?));
    let context = Rc::new(RefCell::new(
        Context::new_with_proplist(mainloop.borrow().deref(), "lush-audio-control", &proplist)
            .ok_or_else(|| "audio control: failed to create context".to_string())?,
    ));

    context
        .borrow_mut()
        .connect(None, ContextFlagSet::NOFLAGS, None)
        .map_err(|e| format!("audio control: connect failed: {:?}", e))?;

    wait_for_ready(&mainloop, &context, CONNECT_TIMEOUT)?;
    let out = f(&mainloop, &context);
    context.borrow_mut().disconnect();
    out
}

fn wait_for_ready(
    mainloop: &Rc<RefCell<Mainloop>>,
    context: &Rc<RefCell<Context>>,
    timeout: Duration,
) -> Result<(), String> {
    let start = Instant::now();
    loop {
        match context.borrow().get_state() {
            ContextState::Ready => return Ok(()),
            ContextState::Failed | ContextState::Terminated => {
                return Err("audio control: context failed before ready".to_string());
            }
            _ => {}
        }

        if start.elapsed() >= timeout {
            return Err("audio control: timed out waiting for ready".to_string());
        }

        iterate_mainloop(mainloop)?;
    }
}

fn query_default_sink_name(
    mainloop: &Rc<RefCell<Mainloop>>,
    context: &Rc<RefCell<Context>>,
) -> Result<String, String> {
    let sink_name = Rc::new(RefCell::new(None::<String>));
    let sink_name_cb = sink_name.clone();
    context.borrow().introspect().get_server_info(move |info| {
        let value = info
            .default_sink_name
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default();
        *sink_name_cb.borrow_mut() = Some(value);
    });

    let start = Instant::now();
    while start.elapsed() < QUERY_TIMEOUT {
        if let Some(value) = sink_name.borrow_mut().take() {
            if value.is_empty() {
                return Err("audio control: default sink is empty".to_string());
            }
            return Ok(value);
        }
        iterate_mainloop(mainloop)?;
    }

    Err("audio control: timed out querying default sink".to_string())
}

fn query_sink_state(
    mainloop: &Rc<RefCell<Mainloop>>,
    context: &Rc<RefCell<Context>>,
    sink_name: &str,
) -> Result<SinkState, String> {
    let sink_state = Rc::new(RefCell::new(None::<SinkState>));
    let sink_state_cb = sink_state.clone();

    context
        .borrow()
        .introspect()
        .get_sink_info_by_name(sink_name, move |result| {
            if let ListResult::Item(info) = result {
                *sink_state_cb.borrow_mut() = Some(SinkState {
                    volume: info.volume,
                    muted: info.mute,
                });
            }
        });

    let start = Instant::now();
    while start.elapsed() < QUERY_TIMEOUT {
        if let Some(value) = sink_state.borrow_mut().take() {
            return Ok(value);
        }
        iterate_mainloop(mainloop)?;
    }

    Err("audio control: timed out querying sink info".to_string())
}

fn apply_sink_volume(
    mainloop: &Rc<RefCell<Mainloop>>,
    context: &Rc<RefCell<Context>>,
    sink_name: &str,
    volume: &ChannelVolumes,
) -> Result<(), String> {
    let done = Rc::new(RefCell::new(None::<bool>));
    let done_cb = done.clone();
    context.borrow_mut().introspect().set_sink_volume_by_name(
        sink_name,
        volume,
        Some(Box::new(move |ok| {
            *done_cb.borrow_mut() = Some(ok);
        })),
    );

    wait_for_apply(mainloop, done, "set sink volume")
}

fn apply_sink_mute(
    mainloop: &Rc<RefCell<Mainloop>>,
    context: &Rc<RefCell<Context>>,
    sink_name: &str,
    muted: bool,
) -> Result<(), String> {
    let done = Rc::new(RefCell::new(None::<bool>));
    let done_cb = done.clone();
    context.borrow_mut().introspect().set_sink_mute_by_name(
        sink_name,
        muted,
        Some(Box::new(move |ok| {
            *done_cb.borrow_mut() = Some(ok);
        })),
    );

    wait_for_apply(mainloop, done, "set sink mute")
}

fn wait_for_apply(
    mainloop: &Rc<RefCell<Mainloop>>,
    done: Rc<RefCell<Option<bool>>>,
    action: &str,
) -> Result<(), String> {
    let start = Instant::now();
    while start.elapsed() < APPLY_TIMEOUT {
        if let Some(ok) = done.borrow_mut().take() {
            if ok {
                return Ok(());
            }
            return Err(format!("audio control: {} failed", action));
        }
        iterate_mainloop(mainloop)?;
    }
    Err(format!("audio control: timed out waiting to {}", action))
}

fn iterate_mainloop(mainloop: &Rc<RefCell<Mainloop>>) -> Result<(), String> {
    match mainloop.borrow_mut().iterate(false) {
        IterateResult::Success(_) => Ok(()),
        IterateResult::Quit(_) | IterateResult::Err(_) => {
            Err("audio control: mainloop iterate failed".to_string())
        }
    }
}
