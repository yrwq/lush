use std::cell::Cell;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct SignalEvent {
    pub name: String,
    pub value: String,
}

type SignalListener = Rc<RefCell<Box<dyn FnMut(&SignalEvent) -> bool>>>;

#[derive(Clone, Default)]
pub struct SignalBus {
    values: Rc<RefCell<HashMap<String, String>>>,
    listeners: Rc<RefCell<BTreeMap<u64, SignalListener>>>,
    keyed_listeners: Rc<RefCell<HashMap<String, BTreeMap<u64, SignalListener>>>>,
    next_listener_id: Rc<Cell<u64>>,
    pending: Rc<RefCell<VecDeque<String>>>,
    pending_set: Rc<RefCell<HashSet<String>>>,
    dispatching: Rc<Cell<bool>>,
    batch_depth: Rc<Cell<u32>>,
}

impl SignalBus {
    pub fn batch<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        self.batch_depth
            .set(self.batch_depth.get().saturating_add(1));
        f();
        self.batch_depth
            .set(self.batch_depth.get().saturating_sub(1));

        if self.batch_depth.get() == 0 && !self.dispatching.get() {
            self.dispatch_pending();
        }
    }

    pub fn set(&self, name: &str, value: &str) {
        let mut values = self.values.borrow_mut();
        let is_same = values.get(name).map(|v| v.as_str()) == Some(value);
        if is_same {
            return;
        }
        values.insert(name.to_string(), value.to_string());
        drop(values);

        self.enqueue_pending(name);

        if self.dispatching.get() || self.batch_depth.get() > 0 {
            return;
        }

        self.dispatch_pending();
    }

    pub fn replay_all(&self) {
        let keys: Vec<String> = self.values.borrow().keys().cloned().collect();
        for name in keys {
            self.enqueue_pending(&name);
        }

        if self.dispatching.get() || self.batch_depth.get() > 0 {
            return;
        }

        self.dispatch_pending();
    }

    fn enqueue_pending(&self, name: &str) {
        let mut pending_set = self.pending_set.borrow_mut();
        let inserted = pending_set.insert(name.to_string());
        drop(pending_set);
        if inserted {
            self.pending.borrow_mut().push_back(name.to_string());
        }
    }

    fn dispatch_pending(&self) {
        self.dispatching.set(true);
        let _guard = DispatchGuard(self.dispatching.clone());
        loop {
            let next = {
                let mut pending = self.pending.borrow_mut();
                pending.pop_front()
            };
            let Some(name) = next else {
                break;
            };
            self.pending_set.borrow_mut().remove(&name);
            let Some(value) = self.values.borrow().get(&name).cloned() else {
                continue;
            };
            let event = SignalEvent { name, value };
            let listeners: Vec<(u64, SignalListener)> = self
                .listeners
                .borrow()
                .iter()
                .map(|(id, cb)| (*id, cb.clone()))
                .collect();
            let keyed_listeners: Vec<(u64, SignalListener)> = self
                .keyed_listeners
                .borrow()
                .get(&event.name)
                .map(|bucket| {
                    bucket
                        .iter()
                        .map(|(id, cb)| (*id, cb.clone()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let mut stale = Vec::new();
            for (id, callback) in listeners {
                if !(callback.borrow_mut())(&event) {
                    stale.push(id);
                }
            }
            let mut stale_keyed = Vec::new();
            for (id, callback) in keyed_listeners {
                if !(callback.borrow_mut())(&event) {
                    stale_keyed.push(id);
                }
            }

            if !stale.is_empty() {
                let mut refs = self.listeners.borrow_mut();
                for id in stale {
                    refs.remove(&id);
                }
            }
            if !stale_keyed.is_empty() {
                let mut buckets = self.keyed_listeners.borrow_mut();
                if let Some(bucket) = buckets.get_mut(&event.name) {
                    for id in stale_keyed {
                        bucket.remove(&id);
                    }
                    if bucket.is_empty() {
                        buckets.remove(&event.name);
                    }
                }
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<String> {
        self.values.borrow().get(name).cloned()
    }

    pub fn snapshot(&self) -> HashMap<String, String> {
        self.values.borrow().clone()
    }

    pub fn subscribe<F>(&self, callback: F) -> u64
    where
        F: FnMut(&SignalEvent) -> bool + 'static,
    {
        let id = self.next_listener_id.get().saturating_add(1);
        self.next_listener_id.set(id);
        self.listeners
            .borrow_mut()
            .insert(id, Rc::new(RefCell::new(Box::new(callback))));
        id
    }

    pub fn subscribe_key<F>(&self, name: &str, callback: F) -> u64
    where
        F: FnMut(&SignalEvent) -> bool + 'static,
    {
        let id = self.next_listener_id.get().saturating_add(1);
        self.next_listener_id.set(id);
        let mut buckets = self.keyed_listeners.borrow_mut();
        buckets
            .entry(name.to_string())
            .or_default()
            .insert(id, Rc::new(RefCell::new(Box::new(callback))));
        id
    }
}

struct DispatchGuard(Rc<Cell<bool>>);

impl Drop for DispatchGuard {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
