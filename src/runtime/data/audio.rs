use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use libpulse_binding as pulse;
use pulse::callbacks::ListResult;
use pulse::context::subscribe::{Facility, InterestMaskSet};
use pulse::context::{Context, FlagSet as ContextFlagSet, State as ContextState};
use pulse::mainloop::standard::{IterateResult, Mainloop};
use pulse::proplist::Proplist;

use super::evented::EventedProvider;
use crate::runtime::signal_bus::SignalBus;

#[derive(Clone, Debug, PartialEq, Eq)]
struct AudioSnapshot {
    volume: u32,
    muted: bool,
    sink: String,
}

pub struct AudioWatcher {
    _evented: EventedProvider,
}

pub fn start_evented(bus: SignalBus) -> Result<AudioWatcher, String> {
    bus.set("data.audio.volume", "0");
    bus.set("data.audio.muted", "0");
    bus.set("data.audio.sink", "");

    let (tx, rx) = async_channel::unbounded::<AudioSnapshot>();
    super::evented::spawn_snapshot_drain(rx, move |snapshot| {
        bus.batch(|| {
            bus.set("data.audio.volume", &snapshot.volume.to_string());
            bus.set("data.audio.muted", if snapshot.muted { "1" } else { "0" });
            bus.set("data.audio.sink", &snapshot.sink);
        });
    });

    let mut evented = EventedProvider::new();
    let stop = evented.stop_flag();
    let thread = thread::spawn(move || {
        if let Err(err) = run_pulse_loop(tx, stop) {
            log::warn!("data.audio: {}", err);
        }
    });
    evented.push_thread(thread);

    Ok(AudioWatcher { _evented: evented })
}

fn run_pulse_loop(
    tx: async_channel::Sender<AudioSnapshot>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<(), String> {
    let mut proplist =
        Proplist::new().ok_or_else(|| "audio: failed to create proplist".to_string())?;
    let _ = proplist.set_str(pulse::proplist::properties::APPLICATION_NAME, "lush");

    let mainloop =
        Rc::new(RefCell::new(Mainloop::new().ok_or_else(|| {
            "audio: failed to create mainloop".to_string()
        })?));
    let context = Rc::new(RefCell::new(
        Context::new_with_proplist(mainloop.borrow().deref(), "lush-audio", &proplist)
            .ok_or_else(|| "audio: failed to create context".to_string())?,
    ));

    context
        .borrow_mut()
        .connect(None, ContextFlagSet::NOFLAGS, None)
        .map_err(|e| format!("audio: failed to connect context: {:?}", e))?;

    wait_for_ready(&mainloop, &context, &stop)?;
    let refresh_pending = Rc::new(Cell::new(true));
    install_subscriptions(&context, refresh_pending.clone());
    let last_snapshot: Rc<RefCell<Option<AudioSnapshot>>> = Rc::new(RefCell::new(None));

    while !stop.load(Ordering::Relaxed) {
        match mainloop.borrow_mut().iterate(false) {
            IterateResult::Success(_) => {}
            IterateResult::Quit(_) | IterateResult::Err(_) => break,
        }
        if refresh_pending.replace(false) {
            refresh_default_sink(&context, tx.clone(), last_snapshot.clone());
        }
        thread::sleep(Duration::from_millis(12));
    }

    Ok(())
}

fn wait_for_ready(
    mainloop: &Rc<RefCell<Mainloop>>,
    context: &Rc<RefCell<Context>>,
    stop: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<(), String> {
    while !stop.load(Ordering::Relaxed) {
        match context.borrow().get_state() {
            ContextState::Ready => return Ok(()),
            ContextState::Failed | ContextState::Terminated => {
                return Err("audio: context failed before ready".to_string());
            }
            _ => match mainloop.borrow_mut().iterate(false) {
                IterateResult::Success(_) => {}
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    return Err("audio: mainloop failed while waiting for ready".to_string());
                }
            },
        }
        thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

fn install_subscriptions(context: &Rc<RefCell<Context>>, refresh_pending: Rc<Cell<bool>>) {
    context.borrow_mut().set_subscribe_callback(Some(Box::new(
        move |facility, _operation, _index| {
            if matches!(facility, Some(Facility::Sink) | Some(Facility::Server)) {
                refresh_pending.set(true);
            }
        },
    )));

    context
        .borrow_mut()
        .subscribe(InterestMaskSet::SINK | InterestMaskSet::SERVER, |_| {});
}

fn refresh_default_sink(
    context: &Rc<RefCell<Context>>,
    tx: async_channel::Sender<AudioSnapshot>,
    last_snapshot: Rc<RefCell<Option<AudioSnapshot>>>,
) {
    let tx_for_server = tx.clone();
    let last_for_server = last_snapshot.clone();
    let context_for_server = context.clone();
    context
        .borrow()
        .introspect()
        .get_server_info(move |server_info| {
            let Some(default_sink_name) = server_info.default_sink_name.as_deref() else {
                maybe_send_snapshot(
                    &tx_for_server,
                    &last_for_server,
                    AudioSnapshot {
                        volume: 0,
                        muted: false,
                        sink: String::new(),
                    },
                );
                return;
            };
            let sink_name = default_sink_name.to_string();
            let sink_name_for_query = sink_name.clone();
            context_for_server
                .borrow()
                .introspect()
                .get_sink_info_by_name(&sink_name_for_query, {
                    let tx = tx.clone();
                    let last_snapshot = last_snapshot.clone();
                    move |result| {
                        if let ListResult::Item(info) = result {
                            let percent = (info.volume.avg().0 as f64
                                / pulse::volume::Volume::NORMAL.0 as f64
                                * 100.0)
                                .round()
                                .clamp(0.0, 150.0) as u32;
                            let sink = info
                                .description
                                .as_ref()
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| sink_name.clone());
                            maybe_send_snapshot(
                                &tx,
                                &last_snapshot,
                                AudioSnapshot {
                                    volume: percent,
                                    muted: info.mute,
                                    sink,
                                },
                            );
                        }
                    }
                });
        });
}

fn maybe_send_snapshot(
    tx: &async_channel::Sender<AudioSnapshot>,
    last_snapshot: &Rc<RefCell<Option<AudioSnapshot>>>,
    snapshot: AudioSnapshot,
) {
    let mut last = last_snapshot.borrow_mut();
    if last.as_ref() == Some(&snapshot) {
        return;
    }
    *last = Some(snapshot.clone());
    let _ = tx.try_send(snapshot);
}
