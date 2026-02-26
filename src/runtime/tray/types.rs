#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrayPixmap {
    pub width: i32,
    pub height: i32,
    pub argb: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrayItemSnapshot {
    pub service: String,
    pub path: String,
    pub iface: String,
    pub id: String,
    pub title: String,
    pub status: String,
    pub icon_name: String,
    pub attention_icon_name: String,
    pub icon_pixmap: Option<TrayPixmap>,
    pub attention_icon_pixmap: Option<TrayPixmap>,
    pub menu_path: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TraySnapshot {
    pub items: Vec<TrayItemSnapshot>,
}

#[derive(Clone, Debug)]
pub struct TrayMenuEntry {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub visible: bool,
    pub separator: bool,
}
