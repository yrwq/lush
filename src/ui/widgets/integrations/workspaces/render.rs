use std::collections::HashMap;

use gtk4::prelude::*;
use gtk4::{Box as GBox, Label};

use crate::ui::widgets::core::common::format_text;
use crate::ui::widgets::core::format::render as render_format;

use super::model::{resolved_icon, resolved_label, workspace_state};

pub(super) fn update_classes(
    tags: &[GBox],
    ids: &[u32],
    focused_mask: u32,
    occupied_mask: u32,
    urgent_mask: u32,
) {
    for (idx, tag) in tags.iter().enumerate() {
        let Some(workspace_id) = ids.get(idx).copied() else {
            tag.set_visible(false);
            tag.remove_css_class("focused");
            tag.remove_css_class("occupied");
            tag.remove_css_class("urgent");
            continue;
        };

        tag.set_visible(true);
        let bit = 1_u32 << (workspace_id - 1);

        if focused_mask & bit != 0 {
            tag.add_css_class("focused");
        } else {
            tag.remove_css_class("focused");
        }

        if occupied_mask & bit != 0 {
            tag.add_css_class("occupied");
        } else {
            tag.remove_css_class("occupied");
        }

        if urgent_mask & bit != 0 {
            tag.add_css_class("urgent");
        } else {
            tag.remove_css_class("urgent");
        }
    }
}

pub(super) struct LabelRenderContext<'a> {
    pub format: Option<&'a str>,
    pub format_states: &'a HashMap<String, String>,
    pub base_labels: &'a [String],
    pub state_labels: &'a HashMap<String, Vec<String>>,
    pub format_icons: &'a HashMap<String, String>,
    pub clients_by_workspace: &'a HashMap<u32, String>,
    pub focused_mask: u32,
    pub occupied_mask: u32,
    pub urgent_mask: u32,
    pub angle: Option<f64>,
}

pub(super) fn update_label_texts(labels: &[Label], ids: &[u32], ctx: &LabelRenderContext<'_>) {
    for (idx, label_widget) in labels.iter().enumerate() {
        let Some(workspace_id) = ids.get(idx).copied() else {
            label_widget.set_text("");
            continue;
        };

        let state = workspace_state(
            workspace_id,
            ctx.focused_mask,
            ctx.occupied_mask,
            ctx.urgent_mask,
        );
        let id = workspace_id.to_string();
        let label = resolved_label(idx, state, ctx.base_labels, ctx.state_labels, &id);
        let icon = resolved_icon(&id, state, ctx.format_icons);
        let clients = ctx
            .clients_by_workspace
            .get(&workspace_id)
            .cloned()
            .unwrap_or_default();
        let rendered = render_format(
            ctx.format,
            ctx.format_states,
            state,
            &[
                ("id", id),
                ("label", label),
                ("icon", icon),
                ("clients", clients),
                ("state", state.to_string()),
            ],
        );
        label_widget.set_text(&format_text(&rendered, ctx.angle));
    }
}
