use gtk4::Box as GBox;
use gtk4::prelude::*;

use crate::runtime::compositor::ToplevelEntry;

pub(super) fn duplicate_rank_in_prefix(
    entries: &[ToplevelEntry],
    idx: usize,
    needle: &ToplevelEntry,
) -> usize {
    let mut rank = 0_usize;
    let needle_app = needle.app_id.trim();
    let needle_title = needle.title.trim();
    for entry in &entries[..=idx] {
        let entry_app = entry.app_id.trim();
        let entry_title = entry.title.trim();
        let app_match = !needle_app.is_empty() && entry_app.eq_ignore_ascii_case(needle_app);
        let title_match =
            !needle_title.is_empty() && entry_title.eq_ignore_ascii_case(needle_title);

        if app_match && title_match {
            rank += 1;
        } else if !needle_title.is_empty() {
            if title_match {
                rank += 1;
            }
        } else if app_match {
            rank += 1;
        }
    }
    rank.saturating_sub(1)
}

pub(super) fn focused_index(
    entries: &[ToplevelEntry],
    focused_title: &str,
    focused_app_id: &str,
) -> Option<usize> {
    if let Some(idx) = entries.iter().position(|entry| entry.focused) {
        return Some(idx);
    }

    let focused_title = focused_title.trim();
    let focused_app_id = focused_app_id.trim();

    if !focused_title.is_empty()
        && !focused_app_id.is_empty()
        && let Some(idx) = entries.iter().position(|entry| {
            let title = entry.title.trim();
            let app_id = entry.app_id.trim();
            !title.is_empty()
                && !app_id.is_empty()
                && title.eq_ignore_ascii_case(focused_title)
                && app_id.eq_ignore_ascii_case(focused_app_id)
        })
    {
        return Some(idx);
    }

    if !focused_title.is_empty()
        && let Some(idx) = entries.iter().position(|entry| {
            let title = entry.title.trim();
            !title.is_empty() && title.eq_ignore_ascii_case(focused_title)
        })
    {
        return Some(idx);
    }

    if !focused_app_id.is_empty() {
        return entries.iter().position(|entry| {
            let app_id = entry.app_id.trim();
            !app_id.is_empty() && app_id.eq_ignore_ascii_case(focused_app_id)
        });
    }

    None
}

pub(super) fn apply_focus_classes(items: &[GBox], focused_idx: Option<usize>) {
    for (idx, item) in items.iter().enumerate() {
        if focused_idx == Some(idx) {
            item.add_css_class("focused");
        } else {
            item.remove_css_class("focused");
        }
    }
}

pub(super) fn display_title(entry: &ToplevelEntry) -> String {
    let title = entry.title.trim();
    if !title.is_empty() {
        return title.to_string();
    }

    let app_id = entry.app_id.trim();
    if !app_id.is_empty() {
        return app_id.to_string();
    }

    "window".to_string()
}
