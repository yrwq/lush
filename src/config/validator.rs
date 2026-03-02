use super::types::{WidgetConfig, WidgetKind, WidgetProps};

pub fn validate_widget_tree(cfg: &WidgetConfig) {
    validate_widget(cfg);
    for child in &cfg.base.children {
        validate_widget_tree(child);
    }
}

fn validate_widget(cfg: &WidgetConfig) {
    warn_disallowed_fields(cfg);

    match (&cfg.kind, &cfg.props) {
        (WidgetKind::CenterBox, _) => {
            if cfg.base.children.len() > 3 {
                log::warn!(
                    "{}centerbox uses only first 3 children (start/center/end); got {}",
                    format_line_prefix(cfg.line),
                    cfg.base.children.len()
                );
            }
        }
        (WidgetKind::Scroll, _) => {
            if cfg.base.children.is_empty() {
                log::warn!(
                    "{}scroll requires one child in 'children'",
                    format_line_prefix(cfg.line),
                );
            } else if cfg.base.children.len() > 1 {
                log::warn!(
                    "{}scroll uses only first child as content; got {}",
                    format_line_prefix(cfg.line),
                    cfg.base.children.len()
                );
            }
        }
        (WidgetKind::Revealer, WidgetProps::Revealer(_)) => {
            if cfg.base.children.is_empty() {
                log::warn!(
                    "{}revealer requires one child in 'children'",
                    format_line_prefix(cfg.line),
                );
            } else if cfg.base.children.len() > 1 {
                log::warn!(
                    "{}revealer uses only first child as content; got {}",
                    format_line_prefix(cfg.line),
                    cfg.base.children.len()
                );
            }
        }
        (WidgetKind::Image, WidgetProps::Image(props)) => {
            let has_path = !props
                .path
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty();
            let has_bind = !props
                .bind
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty();
            if !has_path && !has_bind {
                log::warn!(
                    "{}image requires non-empty 'path' or 'bind'",
                    format_line_prefix(cfg.line),
                );
            }
        }
        (WidgetKind::Progress, WidgetProps::Progress(props)) => {
            let min = props.min.unwrap_or(0.0);
            let max = props.max.unwrap_or(100.0);
            if max <= min {
                log::warn!(
                    "{}progress should use max > min; got min={} max={}",
                    format_line_prefix(cfg.line),
                    min,
                    max
                );
            }
        }
        (WidgetKind::Slider, WidgetProps::Slider(props)) => {
            let min = props.min.unwrap_or(0.0);
            let max = props.max.unwrap_or(100.0);
            if max <= min {
                log::warn!(
                    "{}slider should use max > min; got min={} max={}",
                    format_line_prefix(cfg.line),
                    min,
                    max
                );
            }
            if let Some(step) = props.step
                && step <= 0.0
            {
                log::warn!(
                    "{}slider.step should be > 0; got {}",
                    format_line_prefix(cfg.line),
                    step
                );
            }
            if let Some(scroll_step) = props.scroll_step
                && scroll_step <= 0.0
            {
                log::warn!(
                    "{}slider.scroll_step should be > 0; got {}",
                    format_line_prefix(cfg.line),
                    scroll_step
                );
            }
        }
        (WidgetKind::Entry, WidgetProps::Entry(props)) => {
            if let Some(max_chars) = props.max_chars
                && max_chars <= 0
            {
                log::warn!(
                    "{}entry.max_chars should be > 0; got {}",
                    format_line_prefix(cfg.line),
                    max_chars
                );
            }
        }
        (WidgetKind::List, WidgetProps::List(props)) => {
            if cfg.base.children.is_empty() {
                log::warn!(
                    "{}list requires one template child in 'children'",
                    format_line_prefix(cfg.line),
                );
            } else if cfg.base.children.len() > 1 {
                log::warn!(
                    "{}list uses only first child as template; got {}",
                    format_line_prefix(cfg.line),
                    cfg.base.children.len()
                );
            }
            if let Some(count) = props.count
                && count == 0
            {
                log::warn!(
                    "{}list.count should be > 0; got {}",
                    format_line_prefix(cfg.line),
                    count
                );
            }
        }
        (WidgetKind::Tray, WidgetProps::Tray(props)) => {
            if let Some(max_items) = props.max_items
                && max_items == 0
            {
                log::warn!(
                    "{}tray.max_items should be > 0; got {}",
                    format_line_prefix(cfg.line),
                    max_items
                );
            }
            if let Some(icon_size) = props.icon_size
                && icon_size <= 0
            {
                log::warn!(
                    "{}tray.icon_size should be > 0; got {}",
                    format_line_prefix(cfg.line),
                    icon_size
                );
            }
        }
        (WidgetKind::Popover, _) => {
            if cfg.base.children.is_empty() {
                log::warn!(
                    "{}popover requires children[1] trigger widget and children[2] content widget",
                    format_line_prefix(cfg.line),
                );
            } else if cfg.base.children.len() < 2 {
                log::warn!(
                    "{}popover should define children[1] trigger and children[2] content",
                    format_line_prefix(cfg.line),
                );
            } else if cfg.base.children.len() > 2 {
                log::warn!(
                    "{}popover uses only first 2 children; got {}",
                    format_line_prefix(cfg.line),
                    cfg.base.children.len()
                );
            }
        }
        (WidgetKind::Overlay, _) => {
            if cfg.base.children.is_empty() {
                log::warn!(
                    "{}overlay requires at least one child (base content)",
                    format_line_prefix(cfg.line),
                );
            }
        }
        _ => {}
    }
}

fn warn_disallowed_fields(cfg: &WidgetConfig) {
    let allowed = cfg.allowed_fields();
    let mut present = cfg.base.present_fields();
    present.extend(cfg.props.present_fields());

    for field in present {
        if !allowed.contains(&field) {
            warn_disallowed_field(cfg, field, allowed);
        }
    }
}

fn warn_disallowed_field(cfg: &WidgetConfig, field: &str, allowed: &[&str]) {
    log::warn!(
        "{}{} does not allow field '{}'; allowed fields: [{}]",
        format_line_prefix(cfg.line),
        cfg.kind.as_lua_kind(),
        field,
        allowed.join(", ")
    );
}

fn format_line_prefix(line: Option<i32>) -> String {
    line.map(|l| format!("config.lua:{}: ", l))
        .unwrap_or_default()
}
