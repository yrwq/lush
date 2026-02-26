use std::path::{Path, PathBuf};

use gtk4::CssProvider;

pub struct StyleManager {
    provider: CssProvider,
    css_path: Option<PathBuf>,
}

impl StyleManager {
    pub fn new(path: Option<&str>) -> Self {
        let provider = CssProvider::new();
        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        } else {
            log::warn!("no display available");
        }

        let manager = Self {
            provider,
            css_path: path.map(PathBuf::from),
        };
        if let Err(err) = manager.reload() {
            log::warn!("initial css load failed: {}", err);
        }
        manager
    }

    pub fn reload(&self) -> Result<(), String> {
        let css = match load_css_source(self.css_path.as_deref()) {
            Ok(Some(content)) => format!("{}\n{}", BASE_CSS, content),
            Ok(None) => format!("{}\n{}", BASE_CSS, DEFAULT_CSS),
            Err(err) => return Err(err),
        };
        self.provider.load_from_string(&css);
        Ok(())
    }

    pub fn set_path(&mut self, path: Option<&str>) {
        self.css_path = path.map(PathBuf::from);
    }
}

impl Drop for StyleManager {
    fn drop(&mut self) {
        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_remove_provider_for_display(&display, &self.provider);
        }
    }
}

fn load_css_source(path: Option<&Path>) -> Result<Option<String>, String> {
    let Some(path) = path else {
        return Ok(None);
    };

    std::fs::read_to_string(path)
        .map(Some)
        .map_err(|err| format!("couldn't read css {}: {}", path.display(), err))
}

const BASE_CSS: &str = r#"
window,
.lush-window,
scrolledwindow,
scrolledwindow > viewport,
viewport {
    background: transparent;
    background-image: none;
    box-shadow: none;
}

scrolledwindow undershoot.top,
scrolledwindow undershoot.bottom,
scrolledwindow overshoot.top,
scrolledwindow overshoot.bottom {
    background: transparent;
}

scrollbar.overlay-indicator,
scrollbar.overlay-indicator slider,
scrollbar.overlay-indicator trough {
    opacity: 0;
    min-width: 0;
    min-height: 0;
    background: transparent;
    border: none;
    box-shadow: none;
}
"#;

const DEFAULT_CSS: &str = r#"
* {
    font-size: 13px;
}

.widget.bar {
    padding: 0;
}

.widget {
    margin: 0;
}
"#;
