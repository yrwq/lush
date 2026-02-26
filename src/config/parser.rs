use mlua::{Function, Lua, Result, Table, Value};

use super::types::{
    BoxProps, ButtonProps, ClickAction, ClickBindings, ClockProps, DockClickAction, DockProps,
    IconifyRule, ImageProps, LabelProps, ListProps, OverlayProps, PopoverProps, ProgressProps,
    RevealerProps, RichFormatRule, RichTextStyle, ScrollProps, SliderProps, TrayProps, WidgetBase,
    WidgetConfig, WidgetKind, WidgetProps, WindowConfig, WorkspacesProps,
};
use super::validator::validate_widget_tree;

const GLOBAL_CB_SEQ: &str = "__cb_seq";
const GLOBAL_CALLBACKS: &str = "__callbacks";

const DEFAULT_WINDOW_LAYER: &str = "top";
const DEFAULT_WIDGET_KIND: &str = "hbox";
const WIDGET_WINDOW: &str = "window";

const DEFAULT_TOP_BAR_ANCHORS: [&str; 3] = ["top", "left", "right"];
const POSITION_BOTTOM_ANCHORS: [&str; 3] = ["bottom", "left", "right"];
const POSITION_LEFT_ANCHORS: [&str; 3] = ["left", "top", "bottom"];
const POSITION_RIGHT_ANCHORS: [&str; 3] = ["right", "top", "bottom"];

pub const WIDGET_KINDS: [&str; 18] = [
    "hbox",
    "vbox",
    "centerbox",
    "revealer",
    "scroll",
    "overlay",
    "list",
    "popover",
    "label",
    "button",
    "clock",
    "workspaces",
    "dock",
    "tray",
    "image",
    "progress",
    "slider",
    WIDGET_WINDOW,
];

pub fn parse_windows(lua: &Lua, cfg: &Table) -> Result<Vec<WindowConfig>> {
    let windows_tbl = cfg
        .get::<Table>("windows")
        .map_err(|_| mlua::Error::runtime("config must call lush.ui.windows({...})"))?;

    let mut windows = Vec::new();
    for entry in windows_tbl.sequence_values::<Value>() {
        if let Value::Table(table) = entry? {
            windows.push(parse_window(lua, table)?);
        }
    }

    if windows.is_empty() {
        return Err(mlua::Error::runtime(
            "lush.ui.windows({...}) must contain at least one lush.ui.window(...)",
        ));
    }

    Ok(windows)
}

pub fn widget_ctor(lua: &Lua, kind: &'static str) -> Result<Function> {
    lua.create_function(move |lua, opts: Option<Table>| {
        let desc = lua.create_table()?;
        desc.set("__kind", kind)?;
        if let Some(line) = current_lua_line(lua) {
            desc.set("__line", line)?;
        }

        if let Some(opts) = opts {
            for kv in opts.pairs::<Value, Value>() {
                let (k, v) = kv?;
                desc.set(k, v)?;
            }
        }

        Ok(desc)
    })
}

fn parse_window(lua: &Lua, t: Table) -> Result<WindowConfig> {
    let line = widget_line(&t);
    let mut anchors = anchors_from_position("top");

    if let Ok(anchor_value) = t.get::<Value>("anchor")
        && !matches!(anchor_value, Value::Nil)
    {
        let parsed = parse_anchor(anchor_value, line)?;
        if !parsed.is_empty() {
            anchors = parsed;
        }
    }

    if anchors == anchors_from_position("top")
        && let Ok(pos) = t.get::<String>("position")
    {
        anchors = anchors_from_position(&pos);
    }

    let root_tbl = t
        .get::<Table>("root")
        .map_err(|_| mlua::Error::runtime("window({...}) must define root = <widget>"))?;

    let root = parse_widget(lua, root_tbl)?;
    validate_widget_tree(&root);

    Ok(WindowConfig {
        name: t.get("name").ok(),
        output: parse_output_selector(&t)?,
        visible: t
            .get::<Option<bool>>("visible")
            .unwrap_or(None)
            .unwrap_or(true),
        layer: t
            .get("layer")
            .unwrap_or_else(|_| DEFAULT_WINDOW_LAYER.to_string()),
        exclusive: t.get("exclusive").unwrap_or(false),
        anchors,
        width: t.get("width").ok(),
        height: t.get("height").ok(),
        margin_top: t.get("margin_top").unwrap_or(0),
        margin_bottom: t.get("margin_bottom").unwrap_or(0),
        margin_left: t.get("margin_left").unwrap_or(0),
        margin_right: t.get("margin_right").unwrap_or(0),
        root,
    })
}

fn parse_output_selector(t: &Table) -> Result<Option<String>> {
    match t.get::<Value>("output") {
        Ok(Value::Nil) | Err(_) => Ok(None),
        Ok(Value::Integer(i)) if i >= 0 => Ok(Some(i.to_string())),
        Ok(Value::Number(n)) if n >= 0.0 && n.fract() == 0.0 => Ok(Some((n as u64).to_string())),
        Ok(Value::String(s)) => Ok(Some(s.to_str()?.to_string())),
        _ => Err(mlua::Error::runtime(format!(
            "{}window.output must be a non-negative integer or string",
            format_line_prefix(widget_line(t))
        ))),
    }
}

fn anchors_from_position(pos: &str) -> Vec<String> {
    let values: &[&str] = match pos {
        "bottom" => &POSITION_BOTTOM_ANCHORS,
        "left" => &POSITION_LEFT_ANCHORS,
        "right" => &POSITION_RIGHT_ANCHORS,
        _ => &DEFAULT_TOP_BAR_ANCHORS,
    };
    values.iter().map(|s| (*s).to_string()).collect()
}

fn parse_anchor(v: Value, line: Option<i32>) -> Result<Vec<String>> {
    match v {
        Value::String(s) => Ok(vec![s.to_str()?.to_string()]),
        Value::Table(t) => {
            let mut out = Vec::new();
            for value in t.sequence_values::<Value>() {
                match value? {
                    Value::String(s) => out.push(s.to_str()?.to_string()),
                    _ => {
                        return Err(mlua::Error::runtime(format!(
                            "{}window.anchor table must contain only strings",
                            format_line_prefix(line)
                        )));
                    }
                }
            }
            Ok(out)
        }
        _ => Err(mlua::Error::runtime(format!(
            "{}window.anchor must be a string or an array of strings",
            format_line_prefix(line)
        ))),
    }
}

fn parse_widget(lua: &Lua, t: Table) -> Result<WidgetConfig> {
    let line = widget_line(&t);
    let kind_name = t
        .get::<String>("__kind")
        .unwrap_or_else(|_| DEFAULT_WIDGET_KIND.to_string());
    let Some(kind) = WidgetKind::from_lua_kind(kind_name.as_str()) else {
        return Err(mlua::Error::runtime(format!(
            "{}unknown widget kind '{}'",
            format_line_prefix(line),
            kind_name
        )));
    };

    let base = parse_widget_base(lua, &t)?;
    let props = parse_widget_props(lua, kind, &t)?;

    Ok(WidgetConfig {
        line,
        kind,
        base,
        props,
    })
}

fn parse_widget_base(lua: &Lua, t: &Table) -> Result<WidgetBase> {
    let mut classes = Vec::new();
    if let Ok(single_class) = t.get::<String>("class") {
        classes.push(single_class);
    }
    classes.extend(parse_list_field(t, "classes")?);

    let children = parse_widget_children(lua, t)?;

    Ok(WidgetBase {
        output: parse_output_selector(t)?,
        visible: t.get::<Option<bool>>("visible").unwrap_or(None),
        visible_bind: t.get("visible_bind").ok(),
        width: t.get("width").ok(),
        height: t.get("height").ok(),
        hexpand: t.get::<Option<bool>>("hexpand").unwrap_or(None),
        vexpand: t.get::<Option<bool>>("vexpand").unwrap_or(None),
        halign: t.get("halign").ok(),
        valign: t.get("valign").ok(),
        class_bind: t.get("class_bind").ok(),
        classes,
        children,
    })
}

fn parse_widget_children(lua: &Lua, t: &Table) -> Result<Vec<WidgetConfig>> {
    match t.get::<Value>("children") {
        Ok(Value::Table(list)) => {
            let mut out = Vec::new();
            for entry in list.sequence_values::<Value>() {
                if let Value::Table(child) = entry? {
                    out.push(parse_widget(lua, child)?);
                }
            }
            Ok(out)
        }
        _ => Ok(Vec::new()),
    }
}

fn parse_widget_props(lua: &Lua, kind: WidgetKind, t: &Table) -> Result<WidgetProps> {
    match kind {
        WidgetKind::HBox => Ok(WidgetProps::HBox(parse_box_props(t))),
        WidgetKind::VBox => Ok(WidgetProps::VBox(parse_box_props(t))),
        WidgetKind::CenterBox => Ok(WidgetProps::CenterBox(parse_box_props(t))),
        WidgetKind::Revealer => Ok(WidgetProps::Revealer(parse_revealer_props(t))),
        WidgetKind::Scroll => Ok(WidgetProps::Scroll(parse_scroll_props(t))),
        WidgetKind::Overlay => Ok(WidgetProps::Overlay(parse_overlay_props())),
        WidgetKind::List => Ok(WidgetProps::List(parse_list_props(t))),
        WidgetKind::Popover => Ok(WidgetProps::Popover(parse_popover_props(t))),
        WidgetKind::Label => Ok(WidgetProps::Label(parse_label_props(lua, t)?)),
        WidgetKind::Button => Ok(WidgetProps::Button(parse_button_props(lua, t)?)),
        WidgetKind::Clock => Ok(WidgetProps::Clock(parse_clock_props(t)?)),
        WidgetKind::Workspaces => Ok(WidgetProps::Workspaces(parse_workspaces_props(t)?)),
        WidgetKind::Dock => Ok(WidgetProps::Dock(parse_dock_props(t)?)),
        WidgetKind::Tray => Ok(WidgetProps::Tray(parse_tray_props(t))),
        WidgetKind::Image => Ok(WidgetProps::Image(parse_image_props(lua, t)?)),
        WidgetKind::Progress => Ok(WidgetProps::Progress(parse_progress_props(t))),
        WidgetKind::Slider => Ok(WidgetProps::Slider(parse_slider_props(t))),
    }
}

fn parse_box_props(t: &Table) -> BoxProps {
    BoxProps {
        spacing: t.get("spacing").ok(),
    }
}

fn parse_revealer_props(t: &Table) -> RevealerProps {
    RevealerProps {
        reveal: t.get::<Option<bool>>("reveal").unwrap_or(None),
        reveal_bind: t.get("reveal_bind").ok(),
        transition: t.get("transition").ok(),
        duration: t.get("duration").ok(),
    }
}

fn parse_scroll_props(t: &Table) -> ScrollProps {
    ScrollProps {
        h_policy: t.get("h_policy").ok(),
        v_policy: t.get("v_policy").ok(),
        overlay_scrolling: t.get::<Option<bool>>("overlay_scrolling").unwrap_or(None),
        kinetic_scrolling: t.get::<Option<bool>>("kinetic_scrolling").unwrap_or(None),
        propagate_natural_width: t
            .get::<Option<bool>>("propagate_natural_width")
            .unwrap_or(None),
        propagate_natural_height: t
            .get::<Option<bool>>("propagate_natural_height")
            .unwrap_or(None),
        min_content_width: t.get("min_content_width").ok(),
        min_content_height: t.get("min_content_height").ok(),
    }
}

fn parse_label_props(lua: &Lua, t: &Table) -> Result<LabelProps> {
    Ok(LabelProps {
        text: t.get("text").ok(),
        bind: t.get("bind").ok(),
        binds: parse_map_field(t, "binds")?,
        on_click: parse_on_click_bindings(lua, t)?,
        format: t.get("format").ok(),
        format_states: parse_map_field(t, "format_states")?,
        rules: parse_rules_field(t, "rules")?,
        max_chars: t.get("max_chars").ok(),
        ellipsize: t.get("ellipsize").ok(),
    })
}

fn parse_list_props(t: &Table) -> ListProps {
    ListProps {
        bind: t.get("bind").ok(),
        count: t.get("count").ok(),
        orientation: t.get("orientation").ok(),
        spacing: t.get("spacing").ok(),
    }
}

fn parse_overlay_props() -> OverlayProps {
    OverlayProps {}
}

fn parse_popover_props(t: &Table) -> PopoverProps {
    PopoverProps {
        position: t.get("position").ok(),
        autohide: t.get::<Option<bool>>("autohide").unwrap_or(None),
        has_arrow: t.get::<Option<bool>>("has_arrow").unwrap_or(None),
    }
}

fn parse_button_props(lua: &Lua, t: &Table) -> Result<ButtonProps> {
    Ok(ButtonProps {
        text: t.get("text").ok(),
        bind: t.get("bind").ok(),
        on_click: parse_on_click_bindings(lua, t)?,
        format: t.get("format").ok(),
        format_states: parse_map_field(t, "format_states")?,
        angle: t.get("angle").ok(),
    })
}

fn parse_clock_props(t: &Table) -> Result<ClockProps> {
    Ok(ClockProps {
        format: t.get("format").ok(),
        display_format: t.get("display_format").ok(),
        format_states: parse_map_field(t, "format_states")?,
        interval: t.get("interval").ok(),
        bind: t.get("bind").ok(),
        spacing: t.get("spacing").ok(),
        angle: t.get("angle").ok(),
    })
}

fn parse_workspaces_props(t: &Table) -> Result<WorkspacesProps> {
    Ok(WorkspacesProps {
        count: t.get("count").ok(),
        active_only: t.get::<Option<bool>>("active_only").unwrap_or(None),
        all_outputs: t.get::<Option<bool>>("all_outputs").unwrap_or(None),
        output: parse_output_selector(t)?,
        orientation: t.get("orientation").ok(),
        spacing: t.get("spacing").ok(),
        format: t.get("format").ok(),
        format_states: parse_map_field(t, "format_states")?,
        labels: parse_list_field(t, "labels")?,
        state_labels: parse_list_map_field(t, "state_labels")?,
        format_icons: parse_map_field(t, "format_icons")?,
        show_clients: t.get::<Option<bool>>("show_clients").unwrap_or(None),
        clients_max_items: t.get("clients_max_items").ok(),
        clients_icon_size: t.get("clients_icon_size").ok(),
        clients_rules: parse_iconify_rules_field(t, "clients_rules")?,
        clients_use_glyphs: t.get::<Option<bool>>("clients_use_glyphs").unwrap_or(None),
        clients_glyph_fallback: t.get("clients_glyph_fallback").ok(),
        clients_spacing: t.get("clients_spacing").ok(),
        angle: t.get("angle").ok(),
    })
}

fn parse_image_props(lua: &Lua, t: &Table) -> Result<ImageProps> {
    Ok(ImageProps {
        path: t.get("path").ok(),
        bind: t.get("bind").ok(),
        fit: t.get("fit").ok(),
        can_shrink: t.get::<Option<bool>>("can_shrink").unwrap_or(None),
        on_click: parse_on_click_bindings(lua, t)?,
    })
}

fn parse_dock_props(t: &Table) -> Result<DockProps> {
    Ok(DockProps {
        orientation: t.get("orientation").ok(),
        output: parse_output_selector(t)?,
        all_outputs: t.get::<Option<bool>>("all_outputs").unwrap_or(None),
        spacing: t.get("spacing").ok(),
        max_items: t.get("max_items").ok(),
        format: t.get("format").ok(),
        format_states: parse_map_field(t, "format_states")?,
        image_map: parse_map_field(t, "image_map")?,
        icon_size: t.get("icon_size").ok(),
        on_click: parse_dock_click_bindings(t)?,
        angle: t.get("angle").ok(),
    })
}

fn parse_progress_props(t: &Table) -> ProgressProps {
    ProgressProps {
        bind: t.get("bind").ok(),
        value: t.get("value").ok(),
        min: t.get("min").ok(),
        max: t.get("max").ok(),
        inverted: t.get::<Option<bool>>("inverted").unwrap_or(None),
    }
}

fn parse_tray_props(t: &Table) -> TrayProps {
    TrayProps {
        orientation: t.get("orientation").ok(),
        spacing: t.get("spacing").ok(),
        icon_size: t.get("icon_size").ok(),
        max_items: t.get("max_items").ok(),
        show_passive: t.get::<Option<bool>>("show_passive").unwrap_or(None),
        hide_when_empty: t.get::<Option<bool>>("hide_when_empty").unwrap_or(None),
    }
}

fn parse_slider_props(t: &Table) -> SliderProps {
    SliderProps {
        bind: t.get("bind").ok(),
        value: t.get("value").ok(),
        min: t.get("min").ok(),
        max: t.get("max").ok(),
        step: t.get("step").ok(),
        scroll_step: t.get("scroll_step").ok(),
        orientation: t.get("orientation").ok(),
        inverted: t.get::<Option<bool>>("inverted").unwrap_or(None),
        draw_value: t.get::<Option<bool>>("draw_value").unwrap_or(None),
        digits: t.get("digits").ok(),
    }
}

fn parse_map_field(t: &Table, field: &str) -> Result<std::collections::HashMap<String, String>> {
    match t.get::<Value>(field) {
        Ok(v) => parse_string_map(v),
        Err(_) => Ok(std::collections::HashMap::new()),
    }
}

fn parse_list_map_field(
    t: &Table,
    field: &str,
) -> Result<std::collections::HashMap<String, Vec<String>>> {
    match t.get::<Value>(field) {
        Ok(v) => parse_string_list_map(v),
        Err(_) => Ok(std::collections::HashMap::new()),
    }
}

fn parse_list_field(t: &Table, field: &str) -> Result<Vec<String>> {
    match t.get::<Value>(field) {
        Ok(v) => parse_string_list(v),
        Err(_) => Ok(Vec::new()),
    }
}

fn parse_rules_field(t: &Table, field: &str) -> Result<Vec<RichFormatRule>> {
    match t.get::<Value>(field) {
        Ok(v) => parse_rich_rules(v),
        Err(_) => Ok(Vec::new()),
    }
}

fn parse_iconify_rules_field(t: &Table, field: &str) -> Result<Vec<IconifyRule>> {
    match t.get::<Value>(field) {
        Ok(v) => parse_iconify_rules(v),
        Err(_) => Ok(Vec::new()),
    }
}

fn parse_on_click_bindings(lua: &Lua, t: &Table) -> Result<ClickBindings<ClickAction>> {
    let value = match t.get::<Value>("on_click") {
        Ok(v) => v,
        Err(_) => return Ok(ClickBindings::default()),
    };

    match value {
        Value::Nil => Ok(ClickBindings::default()),
        Value::String(_) | Value::Function(_) => Ok(ClickBindings {
            left: Some(parse_single_click_action(lua, value)?),
            ..ClickBindings::default()
        }),
        Value::Table(map) => {
            let mut out = ClickBindings::default();
            for kv in map.pairs::<Value, Value>() {
                let (key, raw_action) = kv?;
                let key = key
                    .as_string()
                    .ok_or_else(|| {
                        mlua::Error::runtime(format!(
                            "{}on_click table keys must be strings",
                            format_line_prefix(widget_line(t))
                        ))
                    })?
                    .to_str()?
                    .to_ascii_lowercase();
                let parsed = match raw_action {
                    Value::Nil => None,
                    Value::String(_) | Value::Function(_) => {
                        Some(parse_single_click_action(lua, raw_action)?)
                    }
                    _ => {
                        return Err(mlua::Error::runtime(format!(
                            "{}on_click.{} must be a shell command string or function() ... end",
                            format_line_prefix(widget_line(t)),
                            key
                        )));
                    }
                };

                match key.as_str() {
                    "left" => out.left = parsed,
                    "middle" => out.middle = parsed,
                    "right" => out.right = parsed,
                    "wheel_up" => out.wheel_up = parsed,
                    "wheel_down" => out.wheel_down = parsed,
                    _ => {
                        return Err(mlua::Error::runtime(format!(
                            "{}on_click key '{}' is unsupported; use left|middle|right|wheel_up|wheel_down",
                            format_line_prefix(widget_line(t)),
                            key
                        )));
                    }
                }
            }
            Ok(out)
        }
        _ => Err(mlua::Error::runtime(format!(
            "{}on_click must be a shell command string, function() ... end, or a table of button bindings",
            format_line_prefix(widget_line(t))
        ))),
    }
}

fn parse_single_click_action(lua: &Lua, value: Value) -> Result<ClickAction> {
    match value {
        Value::String(s) => Ok(ClickAction::Shell(s.to_str()?.to_string())),
        Value::Function(f) => {
            let id = next_callback_id(lua)?;
            let callbacks: Table = lua.globals().get(GLOBAL_CALLBACKS)?;
            callbacks.set(id, f)?;
            Ok(ClickAction::LuaCallback(id))
        }
        _ => Err(mlua::Error::runtime(
            "click action must be a shell command string or function".to_string(),
        )),
    }
}

fn parse_dock_click_bindings(t: &Table) -> Result<ClickBindings<DockClickAction>> {
    let value = match t.get::<Value>("on_click") {
        Ok(v) => v,
        Err(_) => {
            return Ok(ClickBindings {
                left: Some(DockClickAction::Activate),
                ..ClickBindings::default()
            });
        }
    };

    match value {
        Value::Nil => Ok(ClickBindings {
            left: Some(DockClickAction::Activate),
            ..ClickBindings::default()
        }),
        Value::String(s) => Ok(ClickBindings {
            left: Some(parse_dock_click_action(
                s.to_str()?.as_ref(),
                widget_line(t),
            )?),
            ..ClickBindings::default()
        }),
        Value::Table(map) => {
            let mut out = ClickBindings {
                left: Some(DockClickAction::Activate),
                ..ClickBindings::default()
            };
            for kv in map.pairs::<Value, Value>() {
                let (key, raw_action) = kv?;
                let key = key
                    .as_string()
                    .ok_or_else(|| {
                        mlua::Error::runtime(format!(
                            "{}on_click table keys must be strings",
                            format_line_prefix(widget_line(t))
                        ))
                    })?
                    .to_str()?
                    .to_ascii_lowercase();

                let parsed = match raw_action {
                    Value::Nil => Some(DockClickAction::None),
                    Value::String(s) => Some(parse_dock_click_action(
                        s.to_str()?.as_ref(),
                        widget_line(t),
                    )?),
                    _ => {
                        return Err(mlua::Error::runtime(format!(
                            "{}on_click.{} must be a dock action string",
                            format_line_prefix(widget_line(t)),
                            key
                        )));
                    }
                };

                match key.as_str() {
                    "left" => out.left = parsed,
                    "middle" => out.middle = parsed,
                    "right" => out.right = parsed,
                    "wheel_up" => out.wheel_up = parsed,
                    "wheel_down" => out.wheel_down = parsed,
                    _ => {
                        return Err(mlua::Error::runtime(format!(
                            "{}on_click key '{}' is unsupported; use left|middle|right|wheel_up|wheel_down",
                            format_line_prefix(widget_line(t)),
                            key
                        )));
                    }
                }
            }
            Ok(out)
        }
        _ => Err(mlua::Error::runtime(format!(
            "{}on_click for dock must be a string action or table",
            format_line_prefix(widget_line(t))
        ))),
    }
}

fn parse_dock_click_action(raw: &str, line: Option<i32>) -> Result<DockClickAction> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "none" => Ok(DockClickAction::None),
        "activate" => Ok(DockClickAction::Activate),
        "close" => Ok(DockClickAction::Close),
        "minimize" => Ok(DockClickAction::Minimize),
        "restore" | "unminimize" => Ok(DockClickAction::Restore),
        other => Err(mlua::Error::runtime(format!(
            "{}invalid dock click action '{}'; expected one of: none, activate, close, minimize, restore",
            format_line_prefix(line),
            other
        ))),
    }
}

fn next_callback_id(lua: &Lua) -> Result<u64> {
    let current: u64 = lua.globals().get(GLOBAL_CB_SEQ)?;
    let next = current + 1;
    lua.globals().set(GLOBAL_CB_SEQ, next)?;
    Ok(next)
}

fn parse_string_list(v: Value) -> Result<Vec<String>> {
    match v {
        Value::String(s) => Ok(vec![s.to_str()?.to_string()]),
        Value::Table(t) => {
            let mut out = Vec::new();
            for value in t.sequence_values::<String>() {
                out.push(value?);
            }
            Ok(out)
        }
        _ => Ok(vec![]),
    }
}

fn parse_string_map(v: Value) -> Result<std::collections::HashMap<String, String>> {
    let mut out = std::collections::HashMap::new();
    let Value::Table(t) = v else {
        return Ok(out);
    };

    for kv in t.pairs::<Value, Value>() {
        let (key, value) = kv?;
        let key = match key {
            Value::String(s) => s.to_str()?.to_string(),
            Value::Integer(i) if i >= 0 => i.to_string(),
            Value::Number(n) if n >= 0.0 && n.fract() == 0.0 => (n as u64).to_string(),
            _ => continue,
        };
        let value = match value {
            Value::String(s) => s.to_str()?.to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Number(n) => n.to_string(),
            Value::Boolean(b) => {
                if b {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            _ => continue,
        };
        out.insert(key, value);
    }

    Ok(out)
}

fn parse_string_list_map(v: Value) -> Result<std::collections::HashMap<String, Vec<String>>> {
    let mut out = std::collections::HashMap::new();
    let Value::Table(t) = v else {
        return Ok(out);
    };

    for kv in t.pairs::<Value, Value>() {
        let (key, value) = kv?;
        let key = match key {
            Value::String(s) => s.to_str()?.to_string(),
            Value::Integer(i) if i >= 0 => i.to_string(),
            Value::Number(n) if n >= 0.0 && n.fract() == 0.0 => (n as u64).to_string(),
            _ => continue,
        };
        let list = parse_string_list(value)?;
        if !list.is_empty() {
            out.insert(key, list);
        }
    }

    Ok(out)
}

fn parse_rich_rules(v: Value) -> Result<Vec<RichFormatRule>> {
    let mut out = Vec::new();
    let Value::Table(t) = v else {
        return Ok(out);
    };

    for item in t.sequence_values::<Value>() {
        let Ok(Value::Table(rule_tbl)) = item else {
            continue;
        };
        out.push(RichFormatRule {
            target: rule_tbl.get("target").ok(),
            token: rule_tbl.get("token").ok(),
            r#match: rule_tbl.get("match").ok(),
            min: rule_tbl.get("min").ok(),
            max: rule_tbl.get("max").ok(),
            style: parse_rich_text_style(&rule_tbl)?,
        });
    }

    Ok(out)
}

fn parse_iconify_rules(v: Value) -> Result<Vec<IconifyRule>> {
    let mut out = Vec::new();
    let Value::Table(t) = v else {
        return Ok(out);
    };

    for item in t.sequence_values::<Value>() {
        let Ok(Value::Table(rule_tbl)) = item else {
            continue;
        };
        out.push(IconifyRule {
            class: rule_tbl.get("class").ok(),
            title: rule_tbl.get("title").ok(),
            icon: rule_tbl.get("icon").ok(),
            text: rule_tbl.get("text").ok(),
        });
    }

    Ok(out)
}

fn parse_rich_text_style(t: &Table) -> Result<RichTextStyle> {
    Ok(RichTextStyle {
        format: t.get("format").ok(),
        class_name: t.get("class").ok(),
        color: t.get("color").ok(),
        background: t.get("background").ok(),
        weight: t.get("weight").ok(),
        style: t.get("style").ok(),
        underline: t.get("underline").ok(),
        font: t.get("font").ok(),
        size: t.get("size").ok(),
        rise: t.get("rise").ok(),
        alpha: t.get("alpha").ok(),
        strikethrough: t.get::<Option<bool>>("strikethrough").unwrap_or(None),
    })
}

fn widget_line(t: &Table) -> Option<i32> {
    t.get::<i32>("__line").ok()
}

fn format_line_prefix(line: Option<i32>) -> String {
    line.map(|l| format!("config.lua:{}: ", l))
        .unwrap_or_default()
}

fn current_lua_line(lua: &Lua) -> Option<i32> {
    let debug: Table = lua.globals().get("debug").ok()?;
    let getinfo: Function = debug.get("getinfo").ok()?;
    let info: Table = getinfo.call((2, "l")).ok()?;
    info.get("currentline").ok()
}
