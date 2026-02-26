use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub css: Option<String>,
    pub windows: Vec<WindowConfig>,
}

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub name: Option<String>,
    pub output: Option<String>,
    pub visible: bool,
    pub layer: String,
    pub exclusive: bool,
    pub anchors: Vec<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub margin_top: i32,
    pub margin_bottom: i32,
    pub margin_left: i32,
    pub margin_right: i32,
    pub root: WidgetConfig,
}

#[derive(Debug, Clone, Copy)]
pub enum WidgetKind {
    HBox,
    VBox,
    CenterBox,
    Revealer,
    Scroll,
    Overlay,
    List,
    Popover,
    Label,
    Button,
    Clock,
    Workspaces,
    Dock,
    Tray,
    Image,
    Progress,
    Slider,
}

impl WidgetKind {
    pub fn from_lua_kind(kind: &str) -> Option<Self> {
        match kind {
            "hbox" => Some(Self::HBox),
            "vbox" => Some(Self::VBox),
            "centerbox" => Some(Self::CenterBox),
            "revealer" => Some(Self::Revealer),
            "scroll" => Some(Self::Scroll),
            "overlay" => Some(Self::Overlay),
            "list" => Some(Self::List),
            "popover" => Some(Self::Popover),
            "label" => Some(Self::Label),
            "button" => Some(Self::Button),
            "clock" => Some(Self::Clock),
            "workspaces" => Some(Self::Workspaces),
            "dock" => Some(Self::Dock),
            "tray" => Some(Self::Tray),
            "image" => Some(Self::Image),
            "progress" => Some(Self::Progress),
            "slider" => Some(Self::Slider),
            _ => None,
        }
    }

    pub fn as_lua_kind(self) -> &'static str {
        match self {
            Self::HBox => "hbox",
            Self::VBox => "vbox",
            Self::CenterBox => "centerbox",
            Self::Revealer => "revealer",
            Self::Scroll => "scroll",
            Self::Overlay => "overlay",
            Self::List => "list",
            Self::Popover => "popover",
            Self::Label => "label",
            Self::Button => "button",
            Self::Clock => "clock",
            Self::Workspaces => "workspaces",
            Self::Dock => "dock",
            Self::Tray => "tray",
            Self::Image => "image",
            Self::Progress => "progress",
            Self::Slider => "slider",
        }
    }

    pub fn css_class(self) -> &'static str {
        self.as_lua_kind()
    }

    pub fn allowed_fields(self) -> &'static [&'static str] {
        match self {
            Self::HBox | Self::VBox => BASE_HBOX_VBOX,
            Self::CenterBox => BASE_CENTERBOX,
            Self::Revealer => BASE_REVEALER,
            Self::Scroll => BASE_SCROLL,
            Self::Overlay => BASE_OVERLAY,
            Self::List => BASE_LIST,
            Self::Popover => BASE_POPOVER,
            Self::Label => BASE_LABEL,
            Self::Button => BASE_BUTTON,
            Self::Clock => BASE_CLOCK,
            Self::Workspaces => BASE_WORKSPACES,
            Self::Dock => BASE_DOCK,
            Self::Tray => BASE_TRAY,
            Self::Image => BASE_IMAGE,
            Self::Progress => BASE_PROGRESS,
            Self::Slider => BASE_SLIDER,
        }
    }
}

const BASE_HBOX_VBOX: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "children",
    "spacing",
];

const BASE_CENTERBOX: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "children",
    "spacing",
];

const BASE_SCROLL: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "children",
    "h_policy",
    "v_policy",
    "overlay_scrolling",
    "kinetic_scrolling",
    "propagate_natural_width",
    "propagate_natural_height",
    "min_content_width",
    "min_content_height",
];

const BASE_REVEALER: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "children",
    "reveal",
    "reveal_bind",
    "transition",
    "duration",
];

const BASE_OVERLAY: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "children",
];

const BASE_LIST: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "children",
    "bind",
    "count",
    "orientation",
    "spacing",
];

const BASE_POPOVER: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "children",
    "position",
    "autohide",
    "has_arrow",
];

const BASE_LABEL: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "text",
    "bind",
    "binds",
    "format",
    "format_states",
    "rules",
    "on_click",
    "max_chars",
    "ellipsize",
];

const BASE_BUTTON: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "text",
    "bind",
    "format",
    "format_states",
    "on_click",
    "angle",
];

const BASE_CLOCK: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "format",
    "display_format",
    "format_states",
    "interval",
    "bind",
    "spacing",
    "angle",
];

const BASE_WORKSPACES: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "count",
    "active_only",
    "all_outputs",
    "output",
    "orientation",
    "spacing",
    "format",
    "format_states",
    "labels",
    "state_labels",
    "format_icons",
    "show_clients",
    "clients_max_items",
    "clients_icon_size",
    "clients_rules",
    "clients_use_glyphs",
    "clients_glyph_fallback",
    "clients_spacing",
    "angle",
];

const BASE_DOCK: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "orientation",
    "output",
    "all_outputs",
    "spacing",
    "max_items",
    "format",
    "format_states",
    "image_map",
    "icon_size",
    "on_click",
    "angle",
];

const BASE_TRAY: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "orientation",
    "spacing",
    "icon_size",
    "max_items",
    "show_passive",
    "hide_when_empty",
];

const BASE_IMAGE: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "path",
    "bind",
    "fit",
    "can_shrink",
    "on_click",
];

const BASE_PROGRESS: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "bind",
    "value",
    "min",
    "max",
    "inverted",
];

const BASE_SLIDER: &[&str] = &[
    "class",
    "classes",
    "visible",
    "visible_bind",
    "width",
    "height",
    "hexpand",
    "vexpand",
    "halign",
    "valign",
    "class_bind",
    "bind",
    "value",
    "min",
    "max",
    "step",
    "scroll_step",
    "orientation",
    "inverted",
    "draw_value",
    "digits",
];

#[derive(Debug, Clone)]
pub enum ClickAction {
    Shell(String),
    LuaCallback(u64),
}

#[derive(Debug, Clone)]
pub struct ClickBindings<T> {
    pub left: Option<T>,
    pub middle: Option<T>,
    pub right: Option<T>,
    pub wheel_up: Option<T>,
    pub wheel_down: Option<T>,
}

impl<T> ClickBindings<T> {
    pub fn has_any(&self) -> bool {
        self.left.is_some()
            || self.middle.is_some()
            || self.right.is_some()
            || self.wheel_up.is_some()
            || self.wheel_down.is_some()
    }
}

impl<T> Default for ClickBindings<T> {
    fn default() -> Self {
        Self {
            left: None,
            middle: None,
            right: None,
            wheel_up: None,
            wheel_down: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockClickAction {
    None,
    Activate,
    Close,
    Minimize,
    Restore,
}

#[derive(Debug, Clone, Default)]
pub struct WidgetBase {
    pub output: Option<String>,
    pub visible: Option<bool>,
    pub visible_bind: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub hexpand: Option<bool>,
    pub vexpand: Option<bool>,
    pub halign: Option<String>,
    pub valign: Option<String>,
    pub class_bind: Option<String>,
    pub classes: Vec<String>,
    pub children: Vec<WidgetConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct LabelProps {
    pub text: Option<String>,
    pub bind: Option<String>,
    pub binds: HashMap<String, String>,
    pub on_click: ClickBindings<ClickAction>,
    pub format: Option<String>,
    pub format_states: HashMap<String, String>,
    pub rules: Vec<RichFormatRule>,
    pub max_chars: Option<i32>,
    pub ellipsize: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RichTextStyle {
    pub format: Option<String>,
    pub class_name: Option<String>,
    pub color: Option<String>,
    pub background: Option<String>,
    pub weight: Option<String>,
    pub style: Option<String>,
    pub underline: Option<String>,
    pub font: Option<String>,
    pub size: Option<String>,
    pub rise: Option<String>,
    pub alpha: Option<String>,
    pub strikethrough: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct RichFormatRule {
    pub target: Option<String>,
    pub token: Option<String>,
    pub r#match: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub style: RichTextStyle,
}

#[derive(Debug, Clone, Default)]
pub struct IconifyRule {
    pub class: Option<String>,
    pub title: Option<String>,
    pub icon: Option<String>,
    #[allow(dead_code)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ButtonProps {
    pub text: Option<String>,
    pub bind: Option<String>,
    pub on_click: ClickBindings<ClickAction>,
    pub format: Option<String>,
    pub format_states: HashMap<String, String>,
    pub angle: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct ClockProps {
    pub format: Option<String>,
    pub display_format: Option<String>,
    pub format_states: HashMap<String, String>,
    pub interval: Option<u64>,
    pub bind: Option<String>,
    pub spacing: Option<i32>,
    pub angle: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspacesProps {
    pub count: Option<u32>,
    pub active_only: Option<bool>,
    pub all_outputs: Option<bool>,
    pub output: Option<String>,
    pub orientation: Option<String>,
    pub spacing: Option<i32>,
    pub format: Option<String>,
    pub format_states: HashMap<String, String>,
    pub labels: Vec<String>,
    pub state_labels: HashMap<String, Vec<String>>,
    pub format_icons: HashMap<String, String>,
    pub show_clients: Option<bool>,
    pub clients_max_items: Option<u32>,
    pub clients_icon_size: Option<i32>,
    pub clients_rules: Vec<IconifyRule>,
    pub clients_use_glyphs: Option<bool>,
    pub clients_glyph_fallback: Option<String>,
    pub clients_spacing: Option<i32>,
    pub angle: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct DockProps {
    pub orientation: Option<String>,
    pub output: Option<String>,
    pub all_outputs: Option<bool>,
    pub spacing: Option<i32>,
    pub max_items: Option<u32>,
    pub format: Option<String>,
    pub format_states: HashMap<String, String>,
    pub image_map: HashMap<String, String>,
    pub icon_size: Option<i32>,
    pub on_click: ClickBindings<DockClickAction>,
    pub angle: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct TrayProps {
    pub orientation: Option<String>,
    pub spacing: Option<i32>,
    pub icon_size: Option<i32>,
    pub max_items: Option<u32>,
    pub show_passive: Option<bool>,
    pub hide_when_empty: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct ImageProps {
    pub path: Option<String>,
    pub bind: Option<String>,
    pub fit: Option<String>,
    pub can_shrink: Option<bool>,
    pub on_click: ClickBindings<ClickAction>,
}

#[derive(Debug, Clone, Default)]
pub struct ListProps {
    pub bind: Option<String>,
    pub count: Option<u32>,
    pub orientation: Option<String>,
    pub spacing: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct OverlayProps {}

#[derive(Debug, Clone, Default)]
pub struct PopoverProps {
    pub position: Option<String>,
    pub autohide: Option<bool>,
    pub has_arrow: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct ProgressProps {
    pub bind: Option<String>,
    pub value: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub inverted: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct SliderProps {
    pub bind: Option<String>,
    pub value: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub scroll_step: Option<f64>,
    pub orientation: Option<String>,
    pub inverted: Option<bool>,
    pub draw_value: Option<bool>,
    pub digits: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct BoxProps {
    pub spacing: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct RevealerProps {
    pub reveal: Option<bool>,
    pub reveal_bind: Option<String>,
    pub transition: Option<String>,
    pub duration: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct ScrollProps {
    pub h_policy: Option<String>,
    pub v_policy: Option<String>,
    pub overlay_scrolling: Option<bool>,
    pub kinetic_scrolling: Option<bool>,
    pub propagate_natural_width: Option<bool>,
    pub propagate_natural_height: Option<bool>,
    pub min_content_width: Option<i32>,
    pub min_content_height: Option<i32>,
}

#[derive(Debug, Clone)]
pub enum WidgetProps {
    HBox(BoxProps),
    VBox(BoxProps),
    CenterBox(BoxProps),
    Revealer(RevealerProps),
    Scroll(ScrollProps),
    Overlay(OverlayProps),
    List(ListProps),
    Popover(PopoverProps),
    Label(LabelProps),
    Button(ButtonProps),
    Clock(ClockProps),
    Workspaces(WorkspacesProps),
    Dock(DockProps),
    Tray(TrayProps),
    Image(ImageProps),
    Progress(ProgressProps),
    Slider(SliderProps),
}

#[derive(Debug, Clone)]
pub struct WidgetConfig {
    pub line: Option<i32>,
    pub kind: WidgetKind,
    pub base: WidgetBase,
    pub props: WidgetProps,
}

impl WidgetConfig {
    pub fn allowed_fields(&self) -> &'static [&'static str] {
        self.kind.allowed_fields()
    }
}

pub struct LoadedConfig {
    pub app: AppConfig,
    pub runtime: std::rc::Rc<crate::runtime::lua_runtime::LuaRuntime>,
}
