use glib::MainContext;
use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::{
    Box as GBox, Button, CssProvider, EventControllerScroll, EventControllerScrollFlags,
    GestureClick, Image, Orientation, Popover, STYLE_PROVIDER_PRIORITY_USER, Separator,
    TextDirection, Widget, gdk,
};
use once_cell::sync::Lazy;

use crate::config::{WidgetConfig, WidgetProps};
use crate::runtime::tray;
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::{apply_clickable_cursor, finalize_widget};

struct TrayRenderSettings {
    icon_size: i32,
    max_items: usize,
    show_passive: bool,
    hide_when_empty: bool,
}

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Tray(props) = &cfg.props else {
        unreachable!("tray builder received non-tray props");
    };

    let Some(rx) = tray::subscribe() else {
        let placeholder: Widget = GBox::new(Orientation::Horizontal, 0).upcast();
        placeholder.set_visible(false);
        return placeholder;
    };

    let orientation = match props.orientation.as_deref() {
        Some("vertical") => Orientation::Vertical,
        _ => Orientation::Horizontal,
    };
    let container = GBox::new(orientation, props.spacing.unwrap_or(6));
    container.add_css_class("tray-container");

    let settings = TrayRenderSettings {
        icon_size: props.icon_size.unwrap_or(16).clamp(8, 128),
        max_items: props.max_items.unwrap_or(32).clamp(1, 256) as usize,
        show_passive: props.show_passive.unwrap_or(true),
        hide_when_empty: props.hide_when_empty.unwrap_or(true),
    };

    let render = {
        let container = container.clone();
        let settings = settings;
        move |snapshot: &tray::TraySnapshot| {
            while let Some(child) = container.first_child() {
                container.remove(&child);
            }

            let mut rendered = 0_usize;
            for item in &snapshot.items {
                if !settings.show_passive && item.status.eq_ignore_ascii_case("passive") {
                    continue;
                }
                if rendered >= settings.max_items {
                    break;
                }
                rendered += 1;
                container.append(&build_item(item, settings.icon_size));
            }
            if settings.hide_when_empty {
                container.set_visible(rendered > 0);
            }
        }
    };

    MainContext::default().spawn_local(async move {
        while let Ok(snapshot) = rx.recv().await {
            render(&snapshot);
        }
    });

    let widget: Widget = container.upcast();
    finalize_widget(&widget, cfg, ctx.bus, true);
    widget
}

fn build_item(item: &tray::TrayItemSnapshot, icon_size: i32) -> GBox {
    let button = GBox::new(Orientation::Horizontal, 0);
    button.add_css_class("tray-item");
    button.add_css_class(&format!("tray-{}", sanitize_css_suffix(&item.status)));
    let item_widget: Widget = button.clone().upcast();
    apply_clickable_cursor(&item_widget, true);

    let image = Image::new();
    image.set_pixel_size(icon_size);
    image.set_size_request(icon_size, icon_size);
    image.add_css_class("tray-icon");
    set_image_from_item(&image, item);
    button.append(&image);

    let tooltip = if !item.title.trim().is_empty() {
        Some(item.title.trim())
    } else if !item.id.trim().is_empty() {
        Some(item.id.trim())
    } else {
        None
    };
    button.set_tooltip_text(tooltip);

    bind_click_controller(&button, item.clone());
    bind_scroll_controller(&button, item.clone());
    button
}

fn bind_click_controller(button: &GBox, item: tray::TrayItemSnapshot) {
    let item_for_click = item.clone();
    let anchor_for_click = button.clone();
    let click = GestureClick::new();
    click.set_button(0);
    click.connect_pressed(move |gesture, _, x, y| {
        let xi = x.round() as i32;
        let yi = y.round() as i32;
        let _ = match gesture.current_button() {
            1 => tray::activate(&item_for_click, xi, yi),
            2 => tray::secondary_activate(&item_for_click, xi, yi),
            3 => right_click_menu(&anchor_for_click, &item_for_click, xi, yi),
            _ => Ok(()),
        };
    });
    button.add_controller(click);
}

fn bind_scroll_controller(button: &GBox, item: tray::TrayItemSnapshot) {
    let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
    scroll.connect_scroll(move |_, _, dy| {
        let delta = if dy < 0.0 {
            1
        } else if dy > 0.0 {
            -1
        } else {
            0
        };
        if delta == 0 {
            return glib::Propagation::Proceed;
        }
        let _ = tray::scroll(&item, delta);
        glib::Propagation::Stop
    });
    button.add_controller(scroll);
}

fn set_image_from_item(image: &Image, item: &tray::TrayItemSnapshot) {
    let use_attention = item.status.eq_ignore_ascii_case("needsattention");
    let name = if use_attention {
        item.attention_icon_name.trim()
    } else {
        item.icon_name.trim()
    };

    if !name.is_empty() {
        set_image_from_icon_name(image, name);
        return;
    }

    let pixmap = if use_attention {
        item.attention_icon_pixmap
            .as_ref()
            .or(item.icon_pixmap.as_ref())
    } else {
        item.icon_pixmap
            .as_ref()
            .or(item.attention_icon_pixmap.as_ref())
    };
    let Some(pixmap) = pixmap else {
        set_image_from_icon_name(image, "image-missing-symbolic");
        return;
    };

    let Some(bytes) = argb_to_rgba(&pixmap.argb, pixmap.width, pixmap.height) else {
        set_image_from_icon_name(image, "image-missing-symbolic");
        return;
    };
    let rowstride = pixmap.width.saturating_mul(4);
    let data = glib::Bytes::from_owned(bytes);
    let pixbuf = gdk_pixbuf::Pixbuf::from_bytes(
        &data,
        gdk_pixbuf::Colorspace::Rgb,
        true,
        8,
        pixmap.width,
        pixmap.height,
        rowstride,
    );
    let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
    image.set_paintable(Some(&texture));
}

fn set_image_from_icon_name(image: &Image, icon_name: &str) {
    let size = image.pixel_size().max(1);
    let Some(display) = Display::default() else {
        image.set_paintable(Option::<&gtk4::gdk::Paintable>::None);
        return;
    };
    let theme = gtk4::IconTheme::for_display(&display);
    let icon = theme.lookup_icon(
        icon_name,
        &[],
        size,
        1,
        TextDirection::Ltr,
        gtk4::IconLookupFlags::empty(),
    );
    let Some(file) = icon.file() else {
        image.set_paintable(Option::<&gtk4::gdk::Paintable>::None);
        return;
    };
    let Some(path) = file.path() else {
        image.set_paintable(Option::<&gtk4::gdk::Paintable>::None);
        return;
    };
    let Ok(pixbuf) =
        gdk_pixbuf::Pixbuf::from_file_at_scale(path.to_string_lossy().as_ref(), size, size, true)
    else {
        image.set_paintable(Option::<&gtk4::gdk::Paintable>::None);
        return;
    };
    let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
    image.set_paintable(Some(&texture));
}

fn argb_to_rgba(argb: &[u8], width: i32, height: i32) -> Option<Vec<u8>> {
    if width <= 0 || height <= 0 {
        return None;
    }
    let size = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    if argb.len() < size {
        return None;
    }

    let mut out = vec![0_u8; size];
    for offset in (0..size).step_by(4) {
        out[offset] = argb[offset + 1];
        out[offset + 1] = argb[offset + 2];
        out[offset + 2] = argb[offset + 3];
        out[offset + 3] = argb[offset];
    }
    Some(out)
}

fn sanitize_css_suffix(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn show_dbusmenu_popover(
    anchor: &GBox,
    item: &tray::TrayItemSnapshot,
    entries: &[tray::TrayMenuEntry],
) {
    let popover = Popover::new();
    popover.add_css_class("tray-menu-popover");
    popover.set_has_arrow(false);
    popover.set_autohide(true);
    popover.set_parent(anchor);

    let menu_box = GBox::new(Orientation::Vertical, 2);
    menu_box.add_css_class("tray-menu");

    for entry in entries {
        if !entry.visible {
            continue;
        }
        if entry.separator {
            menu_box.append(&Separator::new(Orientation::Horizontal));
            continue;
        }

        let btn = Button::with_label(&entry.label);
        btn.add_css_class("tray-menu-item");
        btn.set_has_frame(false);
        btn.set_sensitive(entry.enabled);
        btn.set_halign(gtk4::Align::Fill);
        btn.set_hexpand(true);

        let item_for_action = item.clone();
        let popover_for_action = popover.clone();
        let id = entry.id;
        btn.connect_clicked(move |_| {
            let _ = tray::activate_menu_item(&item_for_action, id);
            popover_for_action.popdown();
        });
        menu_box.append(&btn);
    }

    popover.set_child(Some(&menu_box));
    ensure_tray_popup_css_provider();
    popover.popup();
}

fn right_click_menu(
    anchor: &GBox,
    item: &tray::TrayItemSnapshot,
    x: i32,
    y: i32,
) -> Result<(), String> {
    match tray::fetch_menu(item) {
        Ok(entries) if !entries.is_empty() => {
            show_dbusmenu_popover(anchor, item, &entries);
            Ok(())
        }
        _ => tray::context_menu(item, x, y),
    }
}

fn ensure_tray_popup_css_provider() {
    static REGISTERED: Lazy<()> = Lazy::new(|| {
        let provider = CssProvider::new();
        provider.load_from_string(
            "
            .tray-menu-popover,
            .tray-menu-popover > contents,
            .tray-menu-popover contents,
            .tray-menu-popover arrow,
            .tray-menu,
            .tray-menu *,
            .tray-menu-item,
            .tray-menu-item * {
                border-radius: 0;
            }
            ",
        );
        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                STYLE_PROVIDER_PRIORITY_USER,
            );
        }
    });
    Lazy::force(&REGISTERED);
}
