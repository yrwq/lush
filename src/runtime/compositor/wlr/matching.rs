use super::state::{HandleInfo, WlrActionState};

pub(super) fn ordered_candidates<'a>(
    state: &'a WlrActionState,
    app_id: &str,
    title: &str,
) -> Vec<&'a HandleInfo> {
    let app_id_target = app_id.trim();
    let title_target = title.trim();

    let mut tier1 = Vec::new();
    let mut tier2 = Vec::new();
    let mut tier3 = Vec::new();

    for id in &state.order {
        let Some(entry) = state.entries.get(id) else {
            continue;
        };
        let entry_app = entry.app_id.trim();
        let entry_title = entry.title.trim();

        let app_match = !app_id_target.is_empty()
            && !entry_app.is_empty()
            && entry_app.eq_ignore_ascii_case(app_id_target);
        let title_match = !title_target.is_empty()
            && !entry_title.is_empty()
            && entry_title.eq_ignore_ascii_case(title_target);

        if app_match && title_match {
            tier1.push(entry);
        } else if app_match {
            tier2.push(entry);
        } else if title_match {
            tier3.push(entry);
        }
    }

    if !tier1.is_empty() {
        return tier1;
    }
    if !tier2.is_empty() {
        return tier2;
    }
    tier3
}
