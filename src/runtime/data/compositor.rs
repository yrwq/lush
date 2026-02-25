use glib::MainContext;

use crate::runtime::compositor::{self, CompositorStateSnapshot, ToplevelEntry};
use crate::runtime::signal_bus::SignalBus;

pub struct CompositorWatcher {
    task: Option<glib::JoinHandle<()>>,
}

impl Drop for CompositorWatcher {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

pub fn start_evented(bus: SignalBus, output_selector: Option<String>) -> CompositorWatcher {
    bus.batch(|| {
        bus.set("data.compositor.name", compositor::detected_name());
        bus.set("data.compositor.focused_mask", "0");
        bus.set("data.compositor.occupied_mask", "0");
        bus.set("data.compositor.urgent_mask", "0");
        bus.set("data.compositor.focused_workspace", "0");
        bus.set("data.compositor.summary", "focused=0 occupied=0 urgent=0");
        bus.set("data.compositor.focused_window.title", "");
        bus.set("data.compositor.focused_window.app_id", "");
        bus.set("data.compositor.focused_window.workspace", "0");
    });

    let Some(rx) = compositor::subscribe_state(output_selector.as_deref()) else {
        log::info!(
            "data.compositor: no compatible compositor backend detected (name={})",
            compositor::detected_name()
        );
        return CompositorWatcher { task: None };
    };

    let bus_for_task = bus.clone();
    let task = MainContext::default().spawn_local(async move {
        while let Ok(snapshot) = rx.recv().await {
            while let Ok(next) = rx.try_recv() {
                apply_snapshot(&bus_for_task, &next);
            }
            apply_snapshot(&bus_for_task, &snapshot);
        }
    });

    CompositorWatcher { task: Some(task) }
}

fn apply_snapshot(bus: &SignalBus, snapshot: &CompositorStateSnapshot) {
    let workspace = &snapshot.workspace;
    let focused_entry = snapshot.toplevels.iter().find(|entry| entry.focused);
    bus.batch(|| {
        let focused_workspace = workspace.focused_workspace().to_string();
        let focused_title = if let Some(entry) = focused_entry {
            entry.title.trim().to_string()
        } else if snapshot.toplevels.is_empty() {
            workspace.focused_window.title.clone()
        } else {
            String::new()
        };
        let focused_app_id = if let Some(entry) = focused_entry {
            entry.app_id.trim().to_string()
        } else if snapshot.toplevels.is_empty() {
            if workspace.focused_window.app_id.trim().is_empty() {
                app_id_for_focused_title(&focused_title, &snapshot.toplevels).unwrap_or_default()
            } else {
                workspace.focused_window.app_id.clone()
            }
        } else {
            String::new()
        };
        bus.set("data.compositor.name", compositor::detected_name());
        bus.set(
            "data.compositor.focused_mask",
            &workspace.focused_mask.to_string(),
        );
        bus.set(
            "data.compositor.occupied_mask",
            &workspace.occupied_mask.to_string(),
        );
        bus.set(
            "data.compositor.urgent_mask",
            &workspace.urgent_mask.to_string(),
        );
        bus.set("data.compositor.focused_workspace", &focused_workspace);
        bus.set(
            "data.compositor.focused_window.workspace",
            &focused_workspace,
        );
        bus.set("data.compositor.focused_window.title", &focused_title);
        bus.set("data.compositor.focused_window.app_id", &focused_app_id);
        bus.set(
            "data.compositor.summary",
            &format!(
                "focused={} occupied={} urgent={}",
                workspace.focused_mask, workspace.occupied_mask, workspace.urgent_mask
            ),
        );
    });
}

fn app_id_for_focused_title(title: &str, toplevels: &[ToplevelEntry]) -> Option<String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut unique: Option<String> = None;
    for entry in toplevels {
        if !entry.title.trim().eq_ignore_ascii_case(trimmed) {
            continue;
        }
        let app_id = entry.app_id.trim();
        if app_id.is_empty() {
            continue;
        }
        match unique.as_deref() {
            None => unique = Some(app_id.to_string()),
            Some(existing) if existing.eq_ignore_ascii_case(app_id) => {}
            Some(_) => return None,
        }
    }
    unique
}
