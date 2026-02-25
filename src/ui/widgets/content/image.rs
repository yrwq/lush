use std::path::Path;

use gio::File;
use gtk4::prelude::*;
use gtk4::{ContentFit, IconLookupFlags, IconTheme, Picture, TextDirection, Widget};

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

    let picture = Picture::new();
    let width = cfg.base.width;
    let height = cfg.base.height;

    picture.set_can_shrink(props.can_shrink.unwrap_or(true));
    picture.set_content_fit(parse_fit(props.fit.as_deref()));
    set_picture_source(&picture, props.path.as_deref(), width, height);
    wire_image_binding(&picture, props, ctx.bus, width, height);
    wire_gesture_click(&picture, &props.on_click, ctx.loaded, ctx.bus.clone());

    let widget: Widget = picture.upcast();
    finalize_widget(&widget, cfg, ctx.bus, props.on_click.has_any());
    widget
}

fn wire_image_binding(
    picture: &Picture,
    props: &ImageProps,
    bus: &SignalBus,
    width: Option<i32>,
    height: Option<i32>,
) {
    let Some(signal_name) = props.bind.clone() else {
        return;
    };

    if let Some(value) = bus.get(&signal_name) {
        set_picture_source(picture, Some(value.as_str()), width, height);
    }

    let weak = picture.downgrade();
    watch_signal(bus, signal_name, move |value| {
        if let Some(picture) = weak.upgrade() {
            set_picture_source(&picture, Some(value), width, height);
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });
}

fn set_picture_source(
    picture: &Picture,
    source: Option<&str>,
    width: Option<i32>,
    height: Option<i32>,
) {
    let Some(raw) = source.map(str::trim).filter(|s| !s.is_empty()) else {
        picture.set_paintable(Option::<&gtk4::gdk::Texture>::None);
        return;
    };

    if let Some(uri_path) = raw.strip_prefix("file://") {
        let file = File::for_uri(raw);
        if let Some(path) = file.path() {
            if let Some(texture) =
                load_scaled_texture(path.to_string_lossy().as_ref(), width, height)
            {
                picture.set_paintable(Some(&texture));
                return;
            }
        } else if let Some(texture) = load_scaled_texture(uri_path, width, height) {
            picture.set_paintable(Some(&texture));
            return;
        }
        picture.set_file(Some(&file));
        return;
    }

    if raw.contains("://") {
        picture.set_file(Some(&File::for_uri(raw)));
        return;
    }

    if let Some(texture) = load_scaled_texture(raw, width, height) {
        picture.set_paintable(Some(&texture));
        return;
    }

    if Path::new(raw).is_file() {
        picture.set_filename(Some(raw));
        return;
    }

    if let Some(icon) = lookup_icon_paintable(raw, width, height) {
        picture.set_paintable(Some(&icon));
        return;
    }

    picture.set_paintable(Option::<&gtk4::gdk::Texture>::None);
}

fn lookup_icon_paintable(
    icon_name: &str,
    width: Option<i32>,
    height: Option<i32>,
) -> Option<gtk4::IconPaintable> {
    let display = gtk4::gdk::Display::default()?;
    let theme = IconTheme::for_display(&display);
    let size = width.or(height).unwrap_or(48).max(1);
    Some(theme.lookup_icon(
        icon_name,
        &[],
        size,
        1,
        TextDirection::Ltr,
        IconLookupFlags::empty(),
    ))
}

fn load_scaled_texture(
    path: &str,
    width: Option<i32>,
    height: Option<i32>,
) -> Option<gtk4::gdk::Texture> {
    let (w, h) = match (width, height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => (w, h),
        (Some(w), None) if w > 0 => (w, -1),
        (None, Some(h)) if h > 0 => (-1, h),
        _ => return None,
    };

    let preserve_aspect = width.is_none() || height.is_none();
    let pixbuf = gdk_pixbuf::Pixbuf::from_file_at_scale(path, w, h, preserve_aspect).ok()?;
    Some(gtk4::gdk::Texture::for_pixbuf(&pixbuf))
}

fn parse_fit(value: Option<&str>) -> ContentFit {
    match value.unwrap_or("contain") {
        "cover" => ContentFit::Cover,
        "fill" => ContentFit::Fill,
        "scale-down" => ContentFit::ScaleDown,
        _ => ContentFit::Contain,
    }
}
