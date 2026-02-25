mod model;
mod render;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glib::MainContext;
use gtk4::Align;
use gtk4::prelude::*;
use gtk4::{Box as GBox, GestureClick, Image, Label, Orientation, Widget};

use crate::config::{IconifyRule, WidgetConfig, WidgetProps};
use crate::runtime::compositor::{self, ToplevelEntry};
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::{apply_clickable_cursor, finalize_widget};

use self::model::build_display_ids;
use self::render::{LabelRenderContext, update_classes, update_label_texts};

#[derive(Clone)]
struct WorkspaceRenderConfig {
    format: Option<String>,
    format_states: std::collections::HashMap<String, String>,
    labels: Vec<String>,
    state_labels: std::collections::HashMap<String, Vec<String>>,
    icons: std::collections::HashMap<String, String>,
    angle: Option<f64>,
}

#[derive(Clone)]
struct WorkspaceClientsConfig {
    enabled: bool,
    max_items: usize,
    icon_size: i32,
    rules: Vec<IconifyRule>,
    use_glyphs: bool,
    glyph_fallback: Option<String>,
    spacing: i32,
}

fn render_context<'a>(
    cfg: &'a WorkspaceRenderConfig,
    clients_by_workspace: &'a std::collections::HashMap<u32, String>,
    focused_mask: u32,
    occupied_mask: u32,
    urgent_mask: u32,
) -> LabelRenderContext<'a> {
    LabelRenderContext {
        format: cfg.format.as_deref(),
        format_states: &cfg.format_states,
        base_labels: &cfg.labels,
        state_labels: &cfg.state_labels,
        format_icons: &cfg.icons,
        clients_by_workspace,
        focused_mask,
        occupied_mask,
        urgent_mask,
        angle: cfg.angle,
    }
}

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Workspaces(props) = &cfg.props else {
        unreachable!("workspaces builder received non-workspaces props");
    };

    let output_selector = props
        .output
        .as_deref()
        .or(ctx.default_output)
        .map(str::to_string);
    let selector_for_backend = if props.all_outputs.unwrap_or(false) {
        None
    } else {
        output_selector.as_deref()
    };
    let Some(rx) = compositor::subscribe_state(selector_for_backend) else {
        log::debug!("workspaces: compositor workspace backend unavailable; widget disabled");
        let placeholder: Widget = GBox::new(Orientation::Horizontal, 0).upcast();
        placeholder.set_visible(false);
        return placeholder;
    };

    let orientation = match props.orientation.as_deref() {
        Some("vertical") => Orientation::Vertical,
        _ => Orientation::Horizontal,
    };
    let container = GBox::new(orientation, props.spacing.unwrap_or(4));
    container.add_css_class("workspaces-container");
    let workspace_count = props.count.unwrap_or(9).clamp(1, 32);
    let active_only = props.active_only.unwrap_or(false);
    let render_cfg = Rc::new(WorkspaceRenderConfig {
        format: props.format.clone(),
        format_states: props.format_states.clone(),
        labels: props.labels.clone(),
        state_labels: props.state_labels.clone(),
        icons: props.format_icons.clone(),
        angle: props.angle,
    });
    let clients_cfg = Rc::new(WorkspaceClientsConfig {
        enabled: props.show_clients.unwrap_or(false),
        max_items: props.clients_max_items.unwrap_or(4).clamp(1, 16) as usize,
        icon_size: props.clients_icon_size.unwrap_or(12).clamp(8, 64),
        rules: props.clients_rules.clone(),
        use_glyphs: props.clients_use_glyphs.unwrap_or(false),
        glyph_fallback: props.clients_glyph_fallback.clone(),
        spacing: props.clients_spacing.unwrap_or(2).clamp(0, 24),
    });

    let focused_mask = Rc::new(Cell::new(0_u32));
    let occupied_mask = Rc::new(Cell::new(0_u32));
    let urgent_mask = Rc::new(Cell::new(0_u32));
    let entries = Rc::new(RefCell::new(Vec::<ToplevelEntry>::new()));
    let clients_by_workspace =
        Rc::new(RefCell::new(std::collections::HashMap::<u32, String>::new()));
    let display_ids = Rc::new(RefCell::new(build_display_ids(
        workspace_count,
        active_only,
        focused_mask.get(),
        occupied_mask.get(),
        urgent_mask.get(),
    )));

    let mut tags = Vec::with_capacity(workspace_count as usize);
    let mut client_rows = Vec::with_capacity(workspace_count as usize);
    let mut labels = Vec::with_capacity(workspace_count as usize);
    for idx in 0..workspace_count {
        let tag_spacing = if clients_cfg.enabled {
            clients_cfg.spacing
        } else {
            0
        };
        let tag = GBox::new(Orientation::Horizontal, tag_spacing);
        tag.add_css_class("workspace-tag");
        tag.add_css_class(&format!("workspace-slot-{}", idx + 1));
        let clients = GBox::new(Orientation::Horizontal, clients_cfg.spacing);
        clients.add_css_class("workspace-clients");
        clients.set_visible(clients_cfg.enabled);
        clients.set_valign(Align::Center);
        let label = Label::new(None);
        label.set_valign(Align::Center);
        tag.append(&label);
        tag.append(&clients);
        container.append(&tag);
        tags.push(tag);
        client_rows.push(clients);
        labels.push(label);
    }

    for (idx, tag) in tags.iter().enumerate() {
        let tag_widget: Widget = tag.clone().upcast();
        apply_clickable_cursor(&tag_widget, true);

        let ids_for_click = display_ids.clone();
        let focused_for_click = focused_mask.clone();
        let occupied_for_click = occupied_mask.clone();
        let urgent_for_click = urgent_mask.clone();
        let tag_refs_for_click = tags.clone();
        let label_refs_for_click = labels.clone();
        let render_cfg_for_click = render_cfg.clone();
        let clients_for_click = clients_by_workspace.clone();
        let click = GestureClick::new();

        click.connect_pressed(move |_, _, _, _| {
            let ids = ids_for_click.borrow();
            let Some(workspace_id) = ids.get(idx).copied() else {
                return;
            };

            if let Err(err) = compositor::focus_workspace(workspace_id) {
                log::warn!("workspaces: {}", err);
            }

            let mask = 1_u32 << (workspace_id - 1);
            focused_for_click.set(mask);
            update_classes(
                &tag_refs_for_click,
                &ids,
                focused_for_click.get(),
                occupied_for_click.get(),
                urgent_for_click.get(),
            );
            let clients_for_click_ref = clients_for_click.borrow();
            let render_ctx = render_context(
                &render_cfg_for_click,
                &clients_for_click_ref,
                focused_for_click.get(),
                occupied_for_click.get(),
                urgent_for_click.get(),
            );
            update_label_texts(&label_refs_for_click, &ids, &render_ctx);
        });
        tag.add_controller(click);
    }

    let tags_for_updates = tags.clone();
    let client_rows_for_updates = client_rows.clone();
    let focused_for_updates = focused_mask.clone();
    let occupied_for_updates = occupied_mask.clone();
    let urgent_for_updates = urgent_mask.clone();
    let ids_for_updates = display_ids.clone();
    let entries_for_updates = entries.clone();
    let clients_for_updates = clients_by_workspace.clone();
    let labels_for_updates = labels.clone();
    let render_cfg_for_updates = render_cfg.clone();
    let clients_cfg_for_updates = clients_cfg.clone();

    MainContext::default().spawn_local(async move {
        while let Ok(snapshot) = rx.recv().await {
            if snapshot.workspace.focused_mask != 0 {
                focused_for_updates.set(snapshot.workspace.focused_mask);
            }
            occupied_for_updates.set(snapshot.workspace.occupied_mask);
            urgent_for_updates.set(snapshot.workspace.urgent_mask);
            *entries_for_updates.borrow_mut() = snapshot.toplevels;

            let mut ids = ids_for_updates.borrow_mut();
            *ids = build_display_ids(
                workspace_count,
                active_only,
                focused_for_updates.get(),
                occupied_for_updates.get(),
                urgent_for_updates.get(),
            );
            *clients_for_updates.borrow_mut() = build_clients_text_map(
                entries_for_updates.borrow().as_slice(),
                &ids,
                &clients_cfg_for_updates,
            );

            update_classes(
                &tags_for_updates,
                &ids,
                focused_for_updates.get(),
                occupied_for_updates.get(),
                urgent_for_updates.get(),
            );
            let clients_for_updates_ref = clients_for_updates.borrow();
            let render_ctx = render_context(
                &render_cfg_for_updates,
                &clients_for_updates_ref,
                focused_for_updates.get(),
                occupied_for_updates.get(),
                urgent_for_updates.get(),
            );
            update_label_texts(&labels_for_updates, &ids, &render_ctx);
            update_client_icons(
                &client_rows_for_updates,
                &ids,
                entries_for_updates.borrow().as_slice(),
                &clients_cfg_for_updates,
                &snapshot.workspace.focused_window.title,
                &snapshot.workspace.focused_window.app_id,
                &snapshot.workspace.focused_window.identifier,
            );
        }
    });

    {
        let ids = display_ids.borrow();
        *clients_by_workspace.borrow_mut() =
            build_clients_text_map(entries.borrow().as_slice(), &ids, &clients_cfg);
        update_classes(
            &tags,
            &ids,
            focused_mask.get(),
            occupied_mask.get(),
            urgent_mask.get(),
        );
        let clients_ref = clients_by_workspace.borrow();
        let render_ctx = render_context(
            &render_cfg,
            &clients_ref,
            focused_mask.get(),
            occupied_mask.get(),
            urgent_mask.get(),
        );
        update_label_texts(&labels, &ids, &render_ctx);
        update_client_icons(
            &client_rows,
            &ids,
            &entries.borrow(),
            &clients_cfg,
            "",
            "",
            "",
        );
    }

    let widget: Widget = container.upcast();
    finalize_widget(&widget, cfg, ctx.bus, false);
    widget
}

fn build_clients_text_map(
    entries: &[ToplevelEntry],
    workspace_ids: &[u32],
    cfg: &WorkspaceClientsConfig,
) -> std::collections::HashMap<u32, String> {
    let mut out = std::collections::HashMap::new();
    for workspace_id in workspace_ids.iter().copied() {
        let text = build_clients_text_for_workspace(entries, workspace_id, cfg);
        if !text.is_empty() {
            out.insert(workspace_id, text);
        }
    }
    out
}

fn build_clients_text_for_workspace(
    entries: &[ToplevelEntry],
    workspace_id: u32,
    cfg: &WorkspaceClientsConfig,
) -> String {
    if !cfg.use_glyphs {
        return String::new();
    }

    let mut glyphs = Vec::new();
    for entry in entries {
        if !entry_on_workspace(entry, workspace_id) {
            continue;
        }
        let glyph = resolve_client_rule(entry, &cfg.rules)
            .and_then(|rule| rule.icon.clone())
            .or_else(|| cfg.glyph_fallback.clone())
            .or_else(|| {
                let name = entry.app_id.trim();
                if name.is_empty() {
                    None
                } else {
                    name.chars().next().map(|c| c.to_string())
                }
            });
        if let Some(glyph) = glyph
            && !glyph.trim().is_empty()
        {
            glyphs.push(glyph);
        }
        if glyphs.len() >= cfg.max_items {
            break;
        }
    }

    if glyphs.is_empty() {
        String::new()
    } else {
        format!(" {}", glyphs.join(" "))
    }
}

fn update_client_icons(
    rows: &[GBox],
    ids: &[u32],
    entries: &[ToplevelEntry],
    cfg: &WorkspaceClientsConfig,
    focused_title: &str,
    focused_app_id: &str,
    focused_identifier: &str,
) {
    for (idx, row) in rows.iter().enumerate() {
        while let Some(child) = row.first_child() {
            row.remove(&child);
        }

        if !cfg.enabled {
            row.set_visible(false);
            continue;
        }

        let Some(workspace_id) = ids.get(idx).copied() else {
            row.set_visible(false);
            continue;
        };

        let mut count = 0_usize;
        for entry in entries {
            if !entry_on_workspace(entry, workspace_id) {
                continue;
            }
            let is_focused =
                entry_matches_focused(entry, focused_title, focused_app_id, focused_identifier);

            if cfg.use_glyphs {
                let glyph = resolve_client_rule(entry, &cfg.rules)
                    .and_then(|rule| rule.icon.clone())
                    .or_else(|| cfg.glyph_fallback.clone())
                    .or_else(|| {
                        let name = entry.app_id.trim();
                        if name.is_empty() {
                            None
                        } else {
                            name.chars().next().map(|c| c.to_string())
                        }
                    })
                    .unwrap_or_else(|| "?".to_string());
                let text = Label::new(Some(&glyph));
                text.add_css_class("workspace-client-glyph");
                if is_focused {
                    text.add_css_class("focused-client");
                }
                text.set_valign(Align::Center);
                row.append(&text);
            } else {
                let image = Image::new();
                image.add_css_class("workspace-client-icon");
                if is_focused {
                    image.add_css_class("focused-client");
                }
                image.set_pixel_size(cfg.icon_size);
                image.set_valign(Align::Center);
                let source = resolve_client_rule(entry, &cfg.rules)
                    .and_then(|rule| rule.icon.as_deref())
                    .and_then(parse_image_source)
                    .or_else(|| default_image_source(entry));
                match source {
                    Some(ClientImageSource::File(path)) => image.set_from_file(Some(path)),
                    Some(ClientImageSource::IconName(name)) => image.set_icon_name(Some(&name)),
                    None => image.set_icon_name(Some("application-x-executable-symbolic")),
                }
                row.append(&image);
            }

            count += 1;
            if count >= cfg.max_items {
                break;
            }
        }

        row.set_visible(count > 0);
    }
}

enum ClientImageSource {
    File(String),
    IconName(String),
}

fn default_image_source(entry: &ToplevelEntry) -> Option<ClientImageSource> {
    let app_id = entry.app_id.trim();
    for candidate in app_id_candidates(app_id) {
        if let Some(source) = parse_image_source(candidate) {
            return Some(source);
        }
    }

    None
}

fn resolve_client_rule<'a>(
    entry: &ToplevelEntry,
    rules: &'a [IconifyRule],
) -> Option<&'a IconifyRule> {
    if rules.is_empty() {
        return None;
    }

    let app_id = entry.app_id.trim();
    let title = entry.title.trim();
    let mut matched: Option<&IconifyRule> = None;
    for rule in rules {
        let ok_class = match rule.class.as_deref() {
            Some(glob) if !glob.is_empty() => wildcard_match(glob, app_id),
            _ => true,
        };
        let ok_title = match rule.title.as_deref() {
            Some(glob) if !glob.is_empty() => wildcard_match(glob, title),
            _ => true,
        };
        if ok_class && ok_title {
            matched = Some(rule);
        }
    }
    matched
}

fn entry_matches_focused(
    entry: &ToplevelEntry,
    focused_title: &str,
    focused_app_id: &str,
    focused_identifier: &str,
) -> bool {
    if entry.focused {
        return true;
    }

    let title = entry.title.trim();
    let app_id = entry.app_id.trim();
    let focused_title = focused_title.trim();
    let focused_app_id = focused_app_id.trim();
    let focused_identifier = focused_identifier.trim();

    if !focused_identifier.is_empty() {
        let entry_id = entry.identifier.trim();
        if !entry_id.is_empty() {
            return entry_id == focused_identifier;
        }
    }

    if !focused_title.is_empty()
        && !focused_app_id.is_empty()
        && !title.is_empty()
        && !app_id.is_empty()
    {
        return title.eq_ignore_ascii_case(focused_title)
            && app_id.eq_ignore_ascii_case(focused_app_id);
    }
    if !focused_app_id.is_empty() && !app_id.is_empty() {
        return app_id.eq_ignore_ascii_case(focused_app_id);
    }
    false
}

fn entry_on_workspace(entry: &ToplevelEntry, workspace_id: u32) -> bool {
    if (1..=32).contains(&workspace_id) && entry.workspace_mask != 0 {
        let bit = 1_u32 << (workspace_id - 1);
        return entry.workspace_mask & bit != 0;
    }
    entry.workspace_id == Some(workspace_id)
}

fn parse_image_source(value: &str) -> Option<ClientImageSource> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains('/') || std::path::Path::new(trimmed).is_absolute() {
        return Some(ClientImageSource::File(trimmed.to_string()));
    }
    Some(ClientImageSource::IconName(trimmed.to_string()))
}

fn app_id_candidates(app_id: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let trimmed = app_id.trim();
    if trimmed.is_empty() {
        return out;
    }

    out.push(trimmed);
    if let Some(last) = trimmed.rsplit('.').next()
        && !last.is_empty()
        && last != trimmed
    {
        out.push(last);
    }
    out
}

fn wildcard_match(pattern: &str, input: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let p = pattern.as_bytes();
    let s = input.as_bytes();
    let (mut pi, mut si) = (0_usize, 0_usize);
    let (mut star, mut match_i) = (None::<usize>, 0_usize);

    while si < s.len() {
        if pi < p.len() && (p[pi] == s[si] || p[pi] == b'?') {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            pi += 1;
            match_i = si;
        } else if let Some(star_idx) = star {
            pi = star_idx + 1;
            match_i += 1;
            si = match_i;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}
