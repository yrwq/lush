use std::collections::HashMap;

pub(super) fn build_display_ids(
    count: u32,
    active_only: bool,
    focused_mask: u32,
    occupied_mask: u32,
    urgent_mask: u32,
) -> Vec<u32> {
    let count = count.clamp(1, 32) as usize;
    let active_mask = focused_mask | occupied_mask | urgent_mask;

    if active_only {
        let mut out: Vec<u32> = (1..=32)
            .filter(|id| {
                let bit = 1_u32 << (id - 1);
                active_mask & bit != 0
            })
            .collect();
        if out.is_empty()
            && let Some(focused) = first_set_bit(focused_mask)
        {
            out.push(focused);
        }
        out.truncate(count);
        return out;
    }

    let start = first_set_bit(active_mask)
        .map(|id| {
            let block = (id - 1) / (count as u32);
            block * (count as u32) + 1
        })
        .unwrap_or(1);

    let mut out = Vec::with_capacity(count);
    for offset in 0..count {
        let id = start + (offset as u32);
        if id > 32 {
            break;
        }
        out.push(id);
    }
    out
}

pub(super) fn workspace_state(
    id: u32,
    focused_mask: u32,
    occupied_mask: u32,
    urgent_mask: u32,
) -> &'static str {
    let bit = 1_u32 << (id - 1);
    if urgent_mask & bit != 0 {
        "urgent"
    } else if focused_mask & bit != 0 {
        "focused"
    } else if occupied_mask & bit != 0 {
        "occupied"
    } else {
        "unfocused"
    }
}

pub(super) fn resolved_label(
    idx: usize,
    state: &str,
    base_labels: &[String],
    state_labels: &HashMap<String, Vec<String>>,
    fallback_id: &str,
) -> String {
    state_labels
        .get(state)
        .and_then(|values| values.get(idx))
        .filter(|value| !value.is_empty())
        .cloned()
        .or_else(|| {
            state_labels
                .get("default")
                .and_then(|values| values.get(idx))
                .filter(|value| !value.is_empty())
                .cloned()
        })
        .or_else(|| base_labels.get(idx).cloned())
        .unwrap_or_else(|| fallback_id.to_string())
}

pub(super) fn resolved_icon(
    id: &str,
    state: &str,
    format_icons: &HashMap<String, String>,
) -> String {
    format_icons
        .get(&format!("{}.{}", id, state))
        .filter(|value| !value.is_empty())
        .cloned()
        .or_else(|| {
            format_icons
                .get(state)
                .filter(|value| !value.is_empty())
                .cloned()
        })
        .or_else(|| {
            format_icons
                .get(id)
                .filter(|value| !value.is_empty())
                .cloned()
        })
        .or_else(|| {
            format_icons
                .get("default")
                .filter(|value| !value.is_empty())
                .cloned()
        })
        .unwrap_or_default()
}

fn first_set_bit(mask: u32) -> Option<u32> {
    if mask == 0 {
        None
    } else {
        Some(mask.trailing_zeros() + 1)
    }
}
