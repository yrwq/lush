use async_channel::Receiver;

mod control;
mod toplevels;

use crate::runtime::compositor::{CompositorSnapshot, FocusedWindowSnapshot, ToplevelEntry};

pub fn subscribe_snapshots(output_selector: Option<&str>) -> Option<Receiver<CompositorSnapshot>> {
    let rx = crate::runtime::river::subscribe(output_selector)?;
    let (tx, mapped_rx) = async_channel::unbounded();

    glib::MainContext::default().spawn_local(async move {
        let mut last: Option<CompositorSnapshot> = None;
        while let Ok(snapshot) = rx.recv().await {
            let mapped = CompositorSnapshot {
                focused_mask: snapshot.focused,
                occupied_mask: snapshot.occupied,
                urgent_mask: snapshot.urgent,
                focused_window: FocusedWindowSnapshot {
                    title: snapshot.focused_title,
                    app_id: String::new(),
                    identifier: String::new(),
                },
            };

            if last.as_ref() == Some(&mapped) {
                continue;
            }
            last = Some(mapped.clone());

            if tx.send(mapped).await.is_err() {
                break;
            }
        }
    });

    Some(mapped_rx)
}

pub fn subscribe_toplevels(output_selector: Option<&str>) -> Option<Receiver<Vec<ToplevelEntry>>> {
    toplevels::subscribe(output_selector)
}

pub fn focus_workspace(index_1_based: u32) -> Result<(), String> {
    control::focus_workspace(index_1_based)
}

pub fn focus_view_tags() -> Result<(), String> {
    control::focus_view_tags()
}
