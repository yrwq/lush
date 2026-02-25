use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub enum CollectReason {
    Initial,
    Event,
    Interval,
}

#[derive(Debug)]
pub struct EventedProvider {
    stop: Arc<AtomicBool>,
    threads: Vec<thread::JoinHandle<()>>,
}

impl EventedProvider {
    pub fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            threads: Vec::new(),
        }
    }

    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        self.stop.clone()
    }

    pub fn push_thread(&mut self, handle: thread::JoinHandle<()>) {
        self.threads.push(handle);
    }
}

impl Drop for EventedProvider {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        while let Some(handle) = self.threads.pop() {
            let _ = handle.join();
        }
    }
}

pub fn spawn_snapshot_drain<S, F>(rx: async_channel::Receiver<S>, mut apply: F)
where
    S: Send + 'static,
    F: FnMut(S) + 'static,
{
    glib::MainContext::default().spawn_local(async move {
        while let Ok(snapshot) = rx.recv().await {
            apply(snapshot);
        }
    });
}

pub fn trigger_channel() -> (mpsc::SyncSender<()>, mpsc::Receiver<()>) {
    mpsc::sync_channel::<()>(1)
}

pub fn trigger_now(tx: &mpsc::SyncSender<()>) {
    let _ = tx.try_send(());
}

pub fn spawn_periodic_collector<S, C>(
    provider_name: &'static str,
    stop: Arc<AtomicBool>,
    trigger_rx: mpsc::Receiver<()>,
    interval: Duration,
    snapshot_tx: async_channel::Sender<S>,
    mut collect: C,
) -> thread::JoinHandle<()>
where
    S: Send + 'static,
    C: FnMut(CollectReason) -> Result<S, String> + Send + 'static,
{
    thread::spawn(move || {
        let send = |reason: CollectReason,
                    collect: &mut C,
                    snapshot_tx: &async_channel::Sender<S>|
         -> bool {
            match collect(reason) {
                Ok(snapshot) => {
                    if snapshot_tx.send_blocking(snapshot).is_err() {
                        return false;
                    }
                }
                Err(err) => {
                    log::debug!("{} collector error: {}", provider_name, err);
                }
            }
            true
        };

        if !send(CollectReason::Initial, &mut collect, &snapshot_tx) {
            return;
        }

        while !stop.load(Ordering::Relaxed) {
            match trigger_rx.recv_timeout(interval) {
                Ok(()) => {
                    while trigger_rx.try_recv().is_ok() {}
                    if !send(CollectReason::Event, &mut collect, &snapshot_tx) {
                        return;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if !send(CollectReason::Interval, &mut collect, &snapshot_tx) {
                        return;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    })
}

pub fn spawn_udev_trigger_thread(
    stop: Arc<AtomicBool>,
    subsystem: &'static str,
    trigger_tx: mpsc::SyncSender<()>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let Ok(builder) = udev::MonitorBuilder::new() else {
            log::debug!(
                "failed to create udev monitor for subsystem '{}'",
                subsystem
            );
            return;
        };
        let Ok(builder) = builder.match_subsystem(subsystem) else {
            log::debug!("failed to match udev subsystem '{}'", subsystem);
            return;
        };
        let Ok(socket) = builder.listen() else {
            log::debug!("failed to listen on udev subsystem '{}'", subsystem);
            return;
        };
        let fd = socket.as_raw_fd();

        while !stop.load(Ordering::Relaxed) {
            let mut pfd = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: pfd points to one valid pollfd element for the duration of this call.
            let rc = unsafe { libc::poll(&mut pfd, 1, 1000) };
            if rc <= 0 {
                continue;
            }
            if (pfd.revents & libc::POLLIN) == 0 {
                continue;
            }
            for event in socket.iter() {
                let Some(event_subsystem) = event.subsystem() else {
                    continue;
                };
                if event_subsystem == subsystem {
                    trigger_now(&trigger_tx);
                    break;
                }
            }
        }
    })
}
