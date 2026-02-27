use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::runtime::compositor::backends::common;
use async_channel::Sender;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RiverSnapshot {
    pub focused: u32,
    pub occupied: u32,
    pub urgent: u32,
    pub focused_title: String,
    pub view_tags: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct OutputState {
    pub focused: u32,
    pub occupied: u32,
    pub urgent: u32,
}

pub(super) struct SnapshotListener {
    pub selector: Option<String>,
    pub tx: Sender<RiverSnapshot>,
    pub last_sent: Option<RiverSnapshot>,
}

pub(super) type SnapshotListeners = Arc<Mutex<Vec<SnapshotListener>>>;
pub(super) type LatestSnapshotsStore = Arc<Mutex<LatestSnapshots>>;

#[derive(Clone, Debug, Default)]
pub(super) struct LatestSnapshots {
    pub default: RiverSnapshot,
    pub focused_output_name: Option<String>,
    pub by_index: Vec<RiverSnapshot>,
    pub by_global: HashMap<u32, RiverSnapshot>,
    pub by_name: HashMap<String, RiverSnapshot>,
}

pub(super) fn snapshot_from_latest(
    latest: &LatestSnapshots,
    selector: Option<&str>,
) -> RiverSnapshot {
    let Some(selector) = selector else {
        return latest.default.clone();
    };
    if common::selector_is_focused_or_empty(selector) {
        return latest.default.clone();
    }
    if let Some(index) = common::selector_index(selector) {
        return latest
            .by_index
            .get(index)
            .cloned()
            .unwrap_or_else(|| latest.default.clone());
    }
    if let Some(global) = common::selector_global(selector) {
        return latest
            .by_global
            .get(&global)
            .cloned()
            .unwrap_or_else(|| latest.default.clone());
    }

    let Some(target) = common::selector_target_lower(selector) else {
        return latest.default.clone();
    };
    latest
        .by_name
        .get(&target)
        .cloned()
        .unwrap_or_else(|| latest.default.clone())
}

pub(super) fn parse_view_tags(raw: &[u8]) -> Vec<u32> {
    raw.chunks_exact(4)
        .map(|chunk| u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

pub(super) fn union_view_tags(tags: &[u32]) -> u32 {
    tags.iter().copied().fold(0_u32, |acc, mask| acc | mask)
}
