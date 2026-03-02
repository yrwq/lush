use std::path::Path;

use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::{IconLookupFlags, IconTheme, Image, TextDirection, Widget};

use crate::config::{ImageProps, WidgetConfig, WidgetProps};
use crate::runtime::signal_bus::SignalBus;
use crate::ui::signal_watch::watch_signal;
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::clicks::wire_gesture_click;
use crate::ui::widgets::core::common::finalize_widget;

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Image(props) = &cfg.props else {
        unreachable!("image builder received non-image props");
    };

    let image = Image::new();
    let width = cfg.base.width;
    let height = cfg.base.height;
    let size = width.or(height).unwrap_or(48).max(1);

    image.set_pixel_size(size);
    image.set_halign(parse_fit_align(props.fit.as_deref()));
    image.set_valign(parse_fit_align(props.fit.as_deref()));
    set_image_source(&image, props.path.as_deref(), width, height);
    wire_image_binding(&image, props, ctx.bus, width, height);
    wire_gesture_click(&image, &props.on_click, ctx.loaded, ctx.bus.clone());

    let widget: Widget = image.upcast();
    finalize_widget(&widget, cfg, ctx.bus, props.on_click.has_any());
    widget
}

fn wire_image_binding(
    image: &Image,
    props: &ImageProps,
    bus: &SignalBus,
    width: Option<i32>,
    height: Option<i32>,
) {
    let Some(signal_name) = props.bind.clone() else {
        return;
    };

    if let Some(value) = bus.get(&signal_name) {
        set_image_source(image, Some(value.as_str()), width, height);
    }

    let image = image.clone();
    watch_signal(bus, signal_name, move |value| {
        set_image_source(&image, Some(value), width, height);
        glib::ControlFlow::Continue
    });
}

fn set_image_source(image: &Image, source: Option<&str>, width: Option<i32>, height: Option<i32>) {
    let Some(raw) = source.map(str::trim).filter(|s| !s.is_empty()) else {
        image.set_paintable(Option::<&gtk4::gdk::Paintable>::None);
        return;
    };

    if let Some(path) = normalize_file_like_path(raw) {
        if let Some(texture) = load_local_texture(&path, width, height) {
            image.set_paintable(Some(&texture));
            return;
        }
        image.set_paintable(Option::<&gtk4::gdk::Paintable>::None);
        return;
    }

    if let Some(texture) = load_icon_texture(raw, width, height) {
        image.set_paintable(Some(&texture));
        return;
    }

    image.set_paintable(Option::<&gtk4::gdk::Paintable>::None);
}

fn normalize_file_like_path(raw: &str) -> Option<String> {
    if let Some(path) = raw.strip_prefix("file://") {
        if Path::new(path).exists() {
            return Some(path.to_string());
        }
        if let Ok((decoded, _)) = glib::filename_from_uri(raw) {
            return Some(decoded.to_string_lossy().into_owned());
        }
        return None;
    }

    if Path::new(raw).exists() {
        return Some(raw.to_string());
    }

    None
}

fn load_icon_texture(
    icon_name: &str,
    width: Option<i32>,
    height: Option<i32>,
) -> Option<gtk4::gdk::Texture> {
    let display = Display::default()?;
    let theme = IconTheme::for_display(&display);
    let size = width.or(height).unwrap_or(48).max(1);
    let icon = theme.lookup_icon(
        icon_name,
        &[],
        size,
        1,
        TextDirection::Ltr,
        IconLookupFlags::empty(),
    );
    let file = icon.file()?;
    let path = file.path()?;
    load_local_texture(path.to_string_lossy().as_ref(), width, height)
}

fn load_local_texture(
    path: &str,
    width: Option<i32>,
    height: Option<i32>,
) -> Option<gtk4::gdk::Texture> {
    let pixbuf = match (width, height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => {
            gdk_pixbuf::Pixbuf::from_file_at_scale(path, w, h, false).ok()?
        }
        (Some(w), None) if w > 0 => {
            gdk_pixbuf::Pixbuf::from_file_at_scale(path, w, -1, true).ok()?
        }
        (None, Some(h)) if h > 0 => {
            gdk_pixbuf::Pixbuf::from_file_at_scale(path, -1, h, true).ok()?
        }
        _ => gdk_pixbuf::Pixbuf::from_file(path).ok()?,
    };
    Some(gtk4::gdk::Texture::for_pixbuf(&pixbuf))
}

fn parse_fit_align(_value: Option<&str>) -> gtk4::Align {
    gtk4::Align::Center
}
