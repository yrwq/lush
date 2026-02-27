use async_channel::{Receiver, Sender};
use std::sync::{Mutex, MutexGuard};

pub(super) struct BroadcastHub<T>
where
    T: Clone + PartialEq + Send + 'static,
{
    listeners: Mutex<Vec<Sender<T>>>,
    latest: Mutex<Option<T>>,
}

impl<T> BroadcastHub<T>
where
    T: Clone + PartialEq + Send + 'static,
{
    pub(super) fn new() -> Self {
        Self {
            listeners: Mutex::new(Vec::new()),
            latest: Mutex::new(None),
        }
    }

    pub(super) fn subscribe(&self) -> Option<Receiver<T>> {
        let (tx, rx) = async_channel::unbounded();
        if let Some(snapshot) = self.latest_lock().ok()?.clone() {
            let _ = tx.try_send(snapshot);
        }
        self.listeners_lock().ok()?.push(tx);
        Some(rx)
    }

    pub(super) fn publish(&self, snapshot: T) {
        if let Ok(mut latest) = self.latest_lock() {
            if latest.as_ref() == Some(&snapshot) {
                return;
            }
            *latest = Some(snapshot.clone());
        }

        if let Ok(mut listeners) = self.listeners_lock() {
            listeners.retain(|tx| tx.try_send(snapshot.clone()).is_ok());
        }
    }

    fn listeners_lock(&self) -> Result<MutexGuard<'_, Vec<Sender<T>>>, ()> {
        self.listeners.lock().map_err(|_| ())
    }

    fn latest_lock(&self) -> Result<MutexGuard<'_, Option<T>>, ()> {
        self.latest.lock().map_err(|_| ())
    }
}
