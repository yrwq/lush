use std::collections::HashMap;
use std::sync::Mutex;

use async_channel::Sender;

use crate::protocols::ext_foreign_toplevel_list::ext_foreign_toplevel_list_v1::ext_foreign_toplevel_list_v1;

use super::super::ToplevelEntry;

#[derive(Default)]
pub(super) struct ToplevelInfo {
    pub(super) title: String,
    pub(super) app_id: String,
    pub(super) identifier: String,
}

pub(super) struct ToplevelState {
    pub(super) listeners: std::sync::Arc<Mutex<Vec<Sender<Vec<ToplevelEntry>>>>>,
    pub(super) latest: std::sync::Arc<Mutex<Vec<ToplevelEntry>>>,
    pub(super) manager: Option<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1>,
    pub(super) handles: HashMap<wayland_client::backend::ObjectId, ToplevelInfo>,
    pub(super) order: Vec<wayland_client::backend::ObjectId>,
}

impl ToplevelState {
    pub(super) fn new(
        listeners: std::sync::Arc<Mutex<Vec<Sender<Vec<ToplevelEntry>>>>>,
        latest: std::sync::Arc<Mutex<Vec<ToplevelEntry>>>,
    ) -> Self {
        Self {
            listeners,
            latest,
            manager: None,
            handles: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub(super) fn snapshot(&self) -> Vec<ToplevelEntry> {
        let mut out = Vec::with_capacity(self.order.len());
        for id in &self.order {
            let Some(info) = self.handles.get(id) else {
                continue;
            };
            let title = info.title.trim().to_string();
            let app_id = info.app_id.trim().to_string();
            let identifier = info.identifier.trim().to_string();
            if title.is_empty() && app_id.is_empty() {
                continue;
            }
            out.push(ToplevelEntry {
                title,
                app_id,
                identifier,
                workspace_id: None,
                workspace_mask: 0,
                focused: false,
            });
        }
        out
    }

    pub(super) fn emit_updates(&self) {
        let snapshot = self.snapshot();

        if let Ok(mut latest) = self.latest.lock() {
            *latest = snapshot.clone();
        }

        if let Ok(mut listeners) = self.listeners.lock() {
            listeners.retain(|tx| tx.try_send(snapshot.clone()).is_ok());
        }
    }
}
