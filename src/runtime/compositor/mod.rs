use async_channel::Receiver;

mod backends;
mod toplevel;
mod wlr;

use backends::{hyprland, river, sway};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FocusedWindowSnapshot {
    pub title: String,
    pub app_id: String,
    pub identifier: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompositorSnapshot {
    pub focused_mask: u32,
    pub occupied_mask: u32,
    pub urgent_mask: u32,
    pub focused_window: FocusedWindowSnapshot,
}

impl CompositorSnapshot {
    pub fn focused_workspace(&self) -> u32 {
        first_set_bit(self.focused_mask)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToplevelEntry {
    pub title: String,
    pub app_id: String,
    pub identifier: String,
    pub workspace_id: Option<u32>,
    pub workspace_mask: u32,
    pub focused: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompositorStateSnapshot {
    pub workspace: CompositorSnapshot,
    pub toplevels: Vec<ToplevelEntry>,
}

pub fn detected_name() -> &'static str {
    match active_backend() {
        BackendKind::Hyprland => return "hyprland",
        BackendKind::River => return "river",
        BackendKind::Sway => return "sway",
        BackendKind::Unknown => {}
    }

    if let Ok(raw) = std::env::var("XDG_CURRENT_DESKTOP") {
        let lowered = raw.to_ascii_lowercase();
        if lowered.contains("hypr") {
            return "hyprland";
        }
        if lowered.contains("niri") {
            return "niri";
        }
    }

    "unknown"
}

pub fn subscribe_state(output_selector: Option<&str>) -> Option<Receiver<CompositorStateSnapshot>> {
    match active_backend() {
        BackendKind::Hyprland => hyprland::subscribe_state(output_selector)
            .or_else(|| merge_state_streams(None, toplevel::subscribe(output_selector))),
        BackendKind::River => merge_state_streams(
            river::subscribe_snapshots(output_selector),
            river::subscribe_toplevels(output_selector)
                .or_else(|| toplevel::subscribe(output_selector)),
        ),
        BackendKind::Sway => sway::subscribe_state(output_selector),
        BackendKind::Unknown => merge_state_streams(None, toplevel::subscribe(output_selector)),
    }
}

fn merge_state_streams(
    workspace_rx: Option<Receiver<CompositorSnapshot>>,
    toplevel_rx: Option<Receiver<Vec<ToplevelEntry>>>,
) -> Option<Receiver<CompositorStateSnapshot>> {
    if workspace_rx.is_none() && toplevel_rx.is_none() {
        return None;
    }
    enum Update {
        Workspace(CompositorSnapshot),
        Toplevels(Vec<ToplevelEntry>),
    }

    let (update_tx, update_rx) = async_channel::unbounded::<Update>();
    if let Some(rx) = workspace_rx {
        let tx = update_tx.clone();
        glib::MainContext::default().spawn_local(async move {
            while let Ok(snapshot) = rx.recv().await {
                if tx.send(Update::Workspace(snapshot)).await.is_err() {
                    break;
                }
            }
        });
    }
    if let Some(rx) = toplevel_rx {
        let tx = update_tx.clone();
        glib::MainContext::default().spawn_local(async move {
            while let Ok(entries) = rx.recv().await {
                if tx.send(Update::Toplevels(entries)).await.is_err() {
                    break;
                }
            }
        });
    }
    drop(update_tx);

    let (tx, rx) = async_channel::unbounded::<CompositorStateSnapshot>();
    glib::MainContext::default().spawn_local(async move {
        let mut last: Option<CompositorStateSnapshot> = None;
        let mut state = CompositorStateSnapshot::default();
        while let Ok(update) = update_rx.recv().await {
            match update {
                Update::Workspace(snapshot) => state.workspace = snapshot,
                Update::Toplevels(entries) => state.toplevels = entries,
            }
            normalize_state(&mut state);
            if last.as_ref() == Some(&state) {
                continue;
            }
            last = Some(state.clone());
            if tx.send(state.clone()).await.is_err() {
                break;
            }
        }
    });

    Some(rx)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BackendKind {
    Hyprland,
    River,
    Sway,
    Unknown,
}

fn active_backend() -> BackendKind {
    if crate::runtime::river::river_status_available() {
        return BackendKind::River;
    }
    if hyprland::available() {
        return BackendKind::Hyprland;
    }
    if sway::available() {
        return BackendKind::Sway;
    }
    BackendKind::Unknown
}

pub fn focus_workspace(index_1_based: u32) -> Result<(), String> {
    let idx = index_1_based.clamp(1, 32);

    match active_backend() {
        BackendKind::River => return river::focus_workspace(idx),
        BackendKind::Hyprland => return hyprland::focus_workspace(idx),
        BackendKind::Sway => return sway::focus_workspace(idx),
        BackendKind::Unknown => {}
    }

    Err(format!(
        "focus workspace is not implemented for compositor '{}'",
        detected_name()
    ))
}

pub fn activate_toplevel(app_id: &str, title: &str, duplicate_rank: usize) -> Result<(), String> {
    wlr::activate(app_id, title, duplicate_rank)?;

    if detected_name() == "river"
        && let Err(err) = river::focus_view_tags()
    {
        log::debug!("river focus-view-tags after activate failed: {}", err);
    }

    Ok(())
}

pub fn close_toplevel(app_id: &str, title: &str, duplicate_rank: usize) -> Result<(), String> {
    wlr::close(app_id, title, duplicate_rank)
}

pub fn set_toplevel_minimized(
    app_id: &str,
    title: &str,
    duplicate_rank: usize,
    minimized: bool,
) -> Result<(), String> {
    wlr::set_minimized(app_id, title, duplicate_rank, minimized)
}

fn first_set_bit(mask: u32) -> u32 {
    if mask == 0 {
        0
    } else {
        mask.trailing_zeros() + 1
    }
}

fn normalize_state(state: &mut CompositorStateSnapshot) {
    let focused = state.workspace.focused_window.clone();
    let river_backend = active_backend() == BackendKind::River;
    let resolved = resolve_focus_index(&state.toplevels, &focused)
        .or_else(|| resolve_river_focus_index(&state.toplevels, &focused, river_backend));

    if let Some(idx) = resolved {
        set_single_focused(&mut state.toplevels, idx);
        let entry = &state.toplevels[idx];
        state.workspace.focused_window.title = entry.title.clone();
        state.workspace.focused_window.app_id = entry.app_id.clone();
        state.workspace.focused_window.identifier = entry.identifier.clone();
        return;
    }

    for entry in &mut state.toplevels {
        entry.focused = false;
    }

    if !river_backend {
        state.workspace.focused_window.title.clear();
        state.workspace.focused_window.app_id.clear();
        state.workspace.focused_window.identifier.clear();
    }
}

fn resolve_focus_index(
    entries: &[ToplevelEntry],
    focused: &FocusedWindowSnapshot,
) -> Option<usize> {
    if let Some(idx) = entries.iter().position(|entry| entry.focused) {
        return Some(idx);
    }

    let by_id = focused.identifier.trim();
    if !by_id.is_empty()
        && let Some(idx) = entries.iter().position(|entry| {
            let id = entry.identifier.trim();
            !id.is_empty() && id == by_id
        })
    {
        return Some(idx);
    }

    None
}

fn resolve_river_focus_index(
    entries: &[ToplevelEntry],
    focused: &FocusedWindowSnapshot,
    river_backend: bool,
) -> Option<usize> {
    if !river_backend {
        return None;
    }

    let title = focused.title.trim();
    if title.is_empty() {
        return None;
    }

    let mut found: Option<usize> = None;
    for (idx, entry) in entries.iter().enumerate() {
        let entry_title = entry.title.trim();
        if entry_title.is_empty() || !entry_title.eq_ignore_ascii_case(title) {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(idx);
    }
    found
}

fn set_single_focused(entries: &mut [ToplevelEntry], focused_idx: usize) {
    for (idx, entry) in entries.iter_mut().enumerate() {
        entry.focused = idx == focused_idx;
    }
}
