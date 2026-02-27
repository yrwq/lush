use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::broadcast::BroadcastHub;

pub(crate) enum EventLoopStep {
    Refresh,
    Continue,
    Break,
}

pub(crate) fn normalize_selector(output_selector: Option<&str>) -> String {
    output_selector.map(str::trim).unwrap_or("").to_string()
}

pub(crate) fn selector_is_focused_or_empty(selector: &str) -> bool {
    selector.trim().is_empty() || selector.trim().eq_ignore_ascii_case("focused")
}

pub(crate) fn selector_index(selector: &str) -> Option<usize> {
    selector.trim().parse::<usize>().ok()
}

pub(crate) fn selector_global(selector: &str) -> Option<u32> {
    selector.trim().parse::<u32>().ok()
}

pub(crate) fn selector_target_lower(selector: &str) -> Option<String> {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_ascii_lowercase())
}

pub(crate) fn selector_name_eq(name: &str, target_lower: &str) -> bool {
    name.to_ascii_lowercase() == target_lower
}

pub(crate) fn selector_name_contains(name: &str, target_lower: &str) -> bool {
    name.to_ascii_lowercase().contains(target_lower)
}

pub(crate) fn selector_key_and_filter(output_selector: Option<&str>) -> (String, Option<String>) {
    let key = normalize_selector(output_selector);
    let filter = (!key.is_empty()).then_some(key.clone());
    (key, filter)
}

pub(super) fn state_service_for_key<T, Spawn>(
    services: &Mutex<HashMap<String, Arc<BroadcastHub<T>>>>,
    key: &str,
    spawn: Spawn,
) -> Option<Arc<BroadcastHub<T>>>
where
    T: Clone + PartialEq + Send + 'static,
    Spawn: FnOnce(Arc<BroadcastHub<T>>) -> Option<()>,
{
    let mut services = services.lock().ok()?;
    if let Some(existing) = services.get(key) {
        return Some(existing.clone());
    }

    let service = Arc::new(BroadcastHub::new());
    spawn(service.clone())?;
    services.insert(key.to_string(), service.clone());
    Some(service)
}

pub(super) fn run_refresh_loop<T, Collect, Next>(
    service: &Arc<BroadcastHub<T>>,
    mut collect: Collect,
    mut next_step: Next,
) where
    T: Clone + PartialEq + Send + 'static,
    Collect: FnMut() -> Option<T>,
    Next: FnMut() -> EventLoopStep,
{
    if let Some(snapshot) = collect() {
        service.publish(snapshot);
    }

    loop {
        match next_step() {
            EventLoopStep::Refresh => {
                if let Some(snapshot) = collect() {
                    service.publish(snapshot);
                }
            }
            EventLoopStep::Continue => continue,
            EventLoopStep::Break => break,
        }
    }
}
